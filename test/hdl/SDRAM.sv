`timescale 1ns/1ns
// Functional SDR SDRAM model (approx.) for ESMT M12L64322A (2M x 32 @ 200MHz)
// Notes:
//  - Organization: 4 banks x 4096 rows x 128 columns x 32b = 2,097,152 words (2M x 32)
//  - Supports: NOP, ACTIVE, READ, WRITE, PRECHARGE (incl. ALL), AUTO REFRESH (no timing checks), LOAD MODE
//  - CAS latency: 2 or 3 cycles (configurable via mode register)
//  - Burst length: 1 or 4 (mode register), sequential only
//  - Simplified timing checks: basic state gating; detailed tRCD/tRP/tRFC can be extended
module SDRAM #(
    parameter bit SUPPRESS_ERROR_LOGS = 1'b1 // when 1, print violations as $display instead of $error
)(
    input  logic        clk,
    input  logic        cke,
    input  logic        cs_n,
    input  logic        ras_n,
    input  logic        cas_n,
    input  logic        we_n,
    input  logic [1:0]  ba,
    input  logic [12:0] addr,   // A10 used for auto-precharge on READ/WRITE, and precharge-all
    inout  wire  [31:0] dq,
    input  logic [3:0]  dqm     // byte masks
);

    // ---------------------------------------------
    // Parameters (approximate organization)
    // --------------------------------------------- 
    localparam int N_BANKS   = 4;
    localparam int ROW_BITS  = 12; // 4096
    localparam int COL_BITS  = 7;  // 128

    localparam int ROWS      = (1 << ROW_BITS);
    localparam int COLS      = (1 << COL_BITS);

    // Basic timing parameters in cycles (tCK units)
    localparam int tRCD_CYC   = 2; // ACT -> RD/WR
    localparam int tRP_CYC    = 2; // PRE -> ACT (same bank)
    localparam int tRAS_MIN_CYC = 2; // ACT -> PRE minimum
    localparam int tWR_CYC    = 2; // end of WR -> PRE (same bank)
    localparam int tRFC_CYC   = 2; // REF -> next ACT/REF
    localparam int tMRD_CYC   = 2; // MRS -> next command
    localparam int tRRD_CYC   = 2; // ACT -> ACT (any bank)
    localparam int tRC_CYC    = tRAS_MIN_CYC + tRP_CYC + 2; // Row cycle time (ACT->ACT same bank)

    // Mode register fields (subset)
    typedef struct packed {
        logic [2:0] BL; // burst length (0:1, 1:2, 2:4, 3:8; others reserved)
        logic       BT; // burst type (0 sequential; 1 interleaved)
        logic [2:0] CL; // CAS latency (2..3 supported here)
        logic [2:0] OM; // operating mode (unused)
        logic       WB; // write burst mode (0: programmed BL, 1: single)
        logic [3:0] RSVD; // upper bits (unused)
    } mode_reg_t;

    mode_reg_t mode_reg;

    // ---------------------------------------------
    // Memory array (2M x 32)
    // ---------------------------------------------
    logic [31:0] mem [0:N_BANKS-1][0:ROWS-1][0:COLS-1];

    // ---------------------------------------------
    // Bank state
    // ---------------------------------------------
    logic                    bank_active [0:N_BANKS-1];
    logic [ROW_BITS-1:0]     open_row    [0:N_BANKS-1];

    // Timing counters per bank
    int trcd_cnt   [0:N_BANKS-1]; // ACT->RD/WR countdown
    int trp_cnt    [0:N_BANKS-1]; // PRE->ACT countdown
    int tras_min   [0:N_BANKS-1]; // minimum active time left
    int twr_cnt    [0:N_BANKS-1]; // WR->PRE countdown
    int trc_cnt    [0:N_BANKS-1]; // Row cycle time per bank
    int last_act_cycle [0:N_BANKS-1];
    int cycle_count;

    // Global timing counters
    int trfc_cnt; // REF recovery
    int tmrd_cnt; // MRS recovery
    int trrd_cnt; // ACT-to-ACT spacing (global)

    // Snapshot arrays for pre-decrement timing checks
    int trp_pre   [0:N_BANKS-1];
    int tras_pre  [0:N_BANKS-1];
    int twr_pre   [0:N_BANKS-1];
    int trc_pre   [0:N_BANKS-1];

    // Violation tracking (for testbench observation via hierarchy)
    integer violation_count;
    integer last_violation_code; // 1:tRCD_WR, 2:tRCD_RD, 3:tWR, 4:tRP, 5:tRFC, 6:tMRD, 7:tRAS, 8:GlobalBlock
    // 9:tRRD, 10:tRC

    // ---------------------------------------------
    // DQ I/O
    // ---------------------------------------------
    logic        dq_drive_en;
    logic [31:0] dq_out;
    assign dq = dq_drive_en ? dq_out : 32'hzzzz_zzzz;

    // ---------------------------------------------
    // Violation report helper
    // ---------------------------------------------
    task automatic report_violation(input string msg);
        begin
            if (SUPPRESS_ERROR_LOGS) $display("%s", msg);
            else $error("%s", msg);
        end
    endtask

    // ---------------------------------------------
    // Decode commands (when CKE=1)
    // ---------------------------------------------
    wire selected = cke && !cs_n;
    wire do_nop_raw   = !selected || (ras_n && cas_n && we_n);
    wire do_act_raw   = selected && (!ras_n &&  cas_n &&  we_n);
    wire do_read_raw  = selected && ( ras_n && !cas_n &&  we_n);
    wire do_write_raw = selected && ( ras_n && !cas_n && !we_n);
    wire do_pre_raw   = selected && (!ras_n &&  cas_n && !we_n);
    wire do_ref_raw   = selected && (!ras_n && !cas_n &&  we_n);
    wire do_mrs_raw   = selected && (!ras_n && !cas_n && !we_n);

    // Block non-NOP commands during global recovery windows
    wire in_global_block = (trfc_cnt > 0) || (tmrd_cnt > 0);
    wire do_nop   = do_nop_raw || in_global_block;
    wire do_act   = do_act_raw   && !in_global_block;
    wire do_read  = do_read_raw  && !in_global_block;
    wire do_write = do_write_raw && !in_global_block;
    wire do_pre   = do_pre_raw   && !in_global_block;
    wire do_ref   = do_ref_raw   && !in_global_block;
    wire do_mrs   = do_mrs_raw   && !in_global_block;

    // If any non-NOP command is attempted during global block, flag a violation
    always @(posedge clk) begin
        if (cke && !cs_n && in_global_block && !(ras_n && cas_n && we_n)) begin
            report_violation($sformatf("Global recovery violation: command issued during tRFC/tMRD window"));
            violation_count <= violation_count + 1;
            last_violation_code <= 8;
        end
    end

    // ---------------------------------------------
    // READ pipeline entries
    // ---------------------------------------------
    typedef struct packed {
        logic        valid;
        int          cycles_left; // until first data out
        logic [1:0]  bank;
        logic [ROW_BITS-1:0] row;
        logic [COL_BITS-1:0] col;
        logic [COL_BITS-1:0] base_col;
        int          burst_idx;
        int          burst_cnt;
        logic        autopre;
    } read_pipe_t;

    read_pipe_t read_pipe;

    // WRITE tracker (captures data starting next cycle)
    typedef struct packed {
        logic        active;
        logic [1:0]  bank;
        logic [ROW_BITS-1:0] row;
        logic [COL_BITS-1:0] col;
        logic [COL_BITS-1:0] base_col;
        int          burst_idx; // 0-based index within burst
        int          burst_cnt;
        logic        autopre;
    } write_trk_t;

    write_trk_t write_trk;

    // Compute next burst address given base column, index, BL and BT
    function automatic [COL_BITS-1:0] burst_addr(
        input [COL_BITS-1:0] base,
        input int idx,
        input int bl,
        input bit interleaved
    );
        automatic int mask;
        automatic int base_low;
        automatic int low;
        mask = (bl > 1) ? (bl - 1) : 0;
        base_low = base & mask;
        if (interleaved) begin
            // Interleaved: XOR burst index with base low bits
            low = (base_low ^ idx) & mask;
        end else begin
            // Sequential: add burst index with wrap at BL
            low = (base_low + idx) & mask;
        end
        burst_addr = (base & ~mask) | low[COL_BITS-1:0];
    endfunction

    // Utility: compute WRITE burst length in words (WB=1 forces single)
    function int write_burst_len_words(mode_reg_t mr);
        if (mr.WB) return 1;
        unique case (mr.BL)
            3'd0: return 1;
            3'd1: return 2;
            3'd2: return 4;
            3'd3: return 8;
            default: return 1;
        endcase
    endfunction

    // Utility: compute READ burst length in words (ignores WB)
    function int read_burst_len_words(mode_reg_t mr);
        unique case (mr.BL)
            3'd0: return 1;
            3'd1: return 2;
            3'd2: return 4;
            3'd3: return 8;
            default: return 1;
        endcase
    endfunction

    // Utility: CAS latency cycles
    function int cas_latency_cycles(mode_reg_t mr);
        if (mr.CL == 3'd2) return 2; // CL=2
        else return 3;               // default CL=3
    endfunction

    // Reset/init
    initial begin
        integer b;
        for (b = 0; b < N_BANKS; b = b + 1) begin
            bank_active[b] = 1'b0;
            open_row[b]    = '0;
            trcd_cnt[b]    = 0;
            trp_cnt[b]     = 0;
            tras_min[b]    = 0;
            twr_cnt[b]     = 0;
            trc_cnt[b]     = 0;
            last_act_cycle[b] = -1000000;
        end
        cycle_count = 0;
        trfc_cnt = 0;
        tmrd_cnt = 0;
        trrd_cnt = 0;
        violation_count = 0;
        last_violation_code = 0;
        mode_reg = '{BL:3'd0, BT:1'b0, CL:3'd2, OM:3'd0, WB:1'b0, RSVD:4'h0};
        dq_drive_en = 1'b0;
        dq_out      = 32'h0000_0000;
        read_pipe   = '{valid:1'b0, cycles_left:0, bank:'0, row:'0, col:'0, base_col:'0, burst_idx:0, burst_cnt:0, autopre:1'b0};
        write_trk   = '{active:1'b0, bank:'0, row:'0, col:'0, base_col:'0, burst_idx:0, burst_cnt:0, autopre:1'b0};
    end

    // Helpers
    function automatic bit all_banks_precharged();
        int b;
        begin
            all_banks_precharged = 1;
            for (b = 0; b < N_BANKS; b++) begin
                if (bank_active[b]) begin
                    all_banks_precharged = 0;
                end
            end
        end
    endfunction

    // Column increment with sequential burst and wrap
    function automatic [COL_BITS-1:0] next_col(input [COL_BITS-1:0] col, input int bl);
        automatic int mask;
        automatic int col_i;
        // Wrap on BL boundary only (sequential)
        case (bl)
            1: mask = 0;   // no increment
            2: mask = 1;
            4: mask = 3;
            8: mask = 7;
            default: mask = 0;
        endcase
        col_i = col;
        col_i = (col_i & ~mask) | ((col_i + 1) & mask);
        return col_i[COL_BITS-1:0];
    endfunction

    // Main sequential block
    always @(posedge clk) begin
        // Declarations at top of block
        int b;
        logic [31:0] curr;
        logic [31:0] din;
        logic [31:0] mask;
        // advance global cycle counter (after all declarations)
        cycle_count <= cycle_count + 1;
        for (b = 0; b < N_BANKS; b = b + 1) begin
            trp_pre[b]  = trp_cnt[b];
            tras_pre[b] = tras_min[b];
            twr_pre[b]  = twr_cnt[b];
            trc_pre[b]  = trc_cnt[b];
        end

        // Default outputs
        dq_drive_en <= 1'b0;

        // (Decrements moved to end of block; use pre-snapshots for checks)

        // Handle WRITE data capture
        if (write_trk.active) begin
            // Capture one word this cycle (after WRITE cmd was issued previous cycle)
            // Apply byte masks (DQM=1 masks byte write)
            curr = mem[write_trk.bank][write_trk.row][write_trk.col];
            din  = dq; // sampled from bus
            mask[7:0]   = {8{~dqm[0]}};
            mask[15:8]  = {8{~dqm[1]}};
            mask[23:16] = {8{~dqm[2]}};
            mask[31:24] = {8{~dqm[3]}};
            mem[write_trk.bank][write_trk.row][write_trk.col] <= (din & mask) | (curr & ~mask);

            // Prepare next burst element
            write_trk.burst_cnt <= write_trk.burst_cnt - 1;
            if (write_trk.burst_cnt > 1) begin
                write_trk.col <= burst_addr(write_trk.base_col, write_trk.burst_idx, write_burst_len_words(mode_reg), mode_reg.BT);
                write_trk.burst_idx <= write_trk.burst_idx + 1;
            end else begin
                // burst finished
                if (write_trk.autopre) begin
                    bank_active[write_trk.bank] <= 1'b0; // precharge
                    trp_cnt[write_trk.bank]     <= tRP_CYC;
                end
                twr_cnt[write_trk.bank] <= tWR_CYC; // start write recovery
                write_trk.active <= 1'b0;
            end
        end

        // READ data scheduling: first data at exactly CL cycles after READ
        if (read_pipe.valid) begin
            if (read_pipe.cycles_left == 0) begin
                // Drive one data word
                dq_out      <= mem[read_pipe.bank][read_pipe.row][read_pipe.col];
                dq_drive_en <= 1'b1;
                read_pipe.burst_cnt <= read_pipe.burst_cnt - 1;
                if (read_pipe.burst_cnt > 1) begin
                    // Next beat address per mode (sequential/interleaved) using base and index
                    read_pipe.col <= burst_addr(read_pipe.base_col, read_pipe.burst_idx, read_burst_len_words(mode_reg), mode_reg.BT);
                    read_pipe.burst_idx <= read_pipe.burst_idx + 1;
                    read_pipe.cycles_left <= 0; // next beat each clk
                end else begin
                    if (read_pipe.autopre) begin
                        bank_active[read_pipe.bank] <= 1'b0;
                        trp_cnt[read_pipe.bank]     <= tRP_CYC;
                    end
                    read_pipe.valid <= 1'b0;
                end
            end else begin
                read_pipe.cycles_left <= read_pipe.cycles_left - 1;
            end
        end

        // Command handling
        if (do_mrs) begin
            // Load mode register from addr (require all banks precharged)
            if (!all_banks_precharged()) begin
                report_violation($sformatf("MRS requires all banks precharged"));
                violation_count <= violation_count + 1;
                last_violation_code <= 6; // using 6 for MRS-related violation
            end
            mode_reg.BL <= addr[2:0];
            mode_reg.BT <= addr[3];
            mode_reg.CL <= addr[6:4];
            mode_reg.OM <= addr[9:7];
            mode_reg.WB <= addr[9]; // note: overlapping field in some SDRAMs; keep simple
            tmrd_cnt    <= tMRD_CYC;
        end else if (do_act) begin
            // Activate a row in the selected bank
            if (trrd_cnt > 0) begin
                report_violation($sformatf("tRRD violation: ACT to ACT too soon (prev ACT spacing %0d)", trrd_cnt));
                violation_count <= violation_count + 1;
                last_violation_code <= 9;
            end
            if ((cycle_count - last_act_cycle[ba]) < tRC_CYC) begin
                report_violation($sformatf("tRC violation: ACT to ACT too soon on bank %0d (delta=%0d < %0d)", ba, (cycle_count - last_act_cycle[ba]), tRC_CYC));
                violation_count <= violation_count + 1;
                last_violation_code <= 10;
            end
            if (trp_pre[ba] > 0) begin
                report_violation($sformatf("tRP violation: ACT before PRE recovery done for bank %0d", ba));
                violation_count <= violation_count + 1;
                last_violation_code <= 4;
            end
            bank_active[ba] <= 1'b1;
            open_row[ba]    <= addr[ROW_BITS-1:0];
            trcd_cnt[ba]    <= tRCD_CYC; // ACT->R/W latency
            tras_min[ba]    <= tRAS_MIN_CYC; // enforce ACT->PRE minimum
            trrd_cnt        <= tRRD_CYC; // start ACT-to-ACT spacing timer
            trc_cnt[ba]     <= tRC_CYC;  // start row cycle timer for same bank
            last_act_cycle[ba] <= cycle_count;
        end else if (do_pre) begin
            // Precharge: A10=1 => all banks
            if (addr[10]) begin
                for (b = 0; b < N_BANKS; b = b + 1) begin
                    if (!bank_active[b]) continue;
                    // If a write burst is still active on this bank, PRE is a tWR violation
                    if (write_trk.active && write_trk.bank == b) begin
                        report_violation($sformatf("tWR violation: PRE during ongoing WR on bank %0d", b));
                        violation_count <= violation_count + 1;
                        last_violation_code <= 3;
                    end
                    if (tras_pre[b] > 0) begin
                        report_violation($sformatf("tRAS violation: PRE too early on bank %0d", b));
                        violation_count <= violation_count + 1;
                        last_violation_code <= 7;
                    end
                    if (twr_pre[b]  > 0) begin
                        report_violation($sformatf("tWR violation: PRE too soon after WR on bank %0d", b));
                        violation_count <= violation_count + 1;
                        last_violation_code <= 3;
                    end
                    bank_active[b] <= 1'b0;
                    trp_cnt[b]     <= tRP_CYC;
                end
            end else begin
                if (bank_active[ba]) begin
                    if (write_trk.active && write_trk.bank == ba) begin
                        report_violation($sformatf("tWR violation: PRE during ongoing WR on bank %0d", ba));
                        violation_count <= violation_count + 1;
                        last_violation_code <= 3;
                    end
                    if (tras_pre[ba] > 0) begin
                        report_violation($sformatf("tRAS violation: PRE too early on bank %0d", ba));
                        violation_count <= violation_count + 1;
                        last_violation_code <= 7;
                    end
                    if (twr_pre[ba]  > 0) begin
                        report_violation($sformatf("tWR violation: PRE too soon after WR on bank %0d", ba));
                        violation_count <= violation_count + 1;
                        last_violation_code <= 3;
                    end
                    bank_active[ba] <= 1'b0;
                    trp_cnt[ba]     <= tRP_CYC;
                end
            end
        end else if (do_ref) begin
            // Auto-refresh: require all banks precharged
            if (!all_banks_precharged()) begin
                report_violation($sformatf("REF requires all banks precharged"));
                violation_count <= violation_count + 1;
                last_violation_code <= 5;
            end
            trfc_cnt <= tRFC_CYC;
        end else if (do_write) begin
            // Require bank active and tRCD satisfied
            if (bank_active[ba] && trcd_cnt[ba] == 0) begin
                write_trk.active    <= 1'b1;
                write_trk.bank      <= ba;
                write_trk.row       <= open_row[ba];
                write_trk.base_col  <= addr[COL_BITS-1:0];
                write_trk.col       <= addr[COL_BITS-1:0]; // first beat at base_col
                write_trk.burst_idx <= 1; // next index to use
                write_trk.burst_cnt <= write_burst_len_words(mode_reg);
                write_trk.autopre   <= addr[10];
            end else if (bank_active[ba]) begin
                report_violation($sformatf("tRCD violation: WR too soon after ACT on bank %0d", ba));
                violation_count <= violation_count + 1;
                last_violation_code <= 1;
            end
        end else if (do_read) begin
            if (bank_active[ba] && trcd_cnt[ba] == 0) begin
                read_pipe.valid       <= 1'b1;
                read_pipe.cycles_left <= (cas_latency_cycles(mode_reg) > 0) ? cas_latency_cycles(mode_reg) - 1 : 0;
                read_pipe.bank        <= ba;
                read_pipe.row         <= open_row[ba];
                read_pipe.base_col    <= addr[COL_BITS-1:0];
                read_pipe.col         <= addr[COL_BITS-1:0];
                read_pipe.burst_idx   <= 1;
                read_pipe.burst_cnt   <= read_burst_len_words(mode_reg);
                read_pipe.autopre     <= addr[10];
            end else if (bank_active[ba]) begin
                report_violation($sformatf("tRCD violation: RD too soon after ACT on bank %0d", ba));
                violation_count <= violation_count + 1;
                last_violation_code <= 2;
            end
        end

        // Now decrement timing counters at end of cycle
        for (b = 0; b < N_BANKS; b = b + 1) begin
            if (trcd_cnt[b] > 0) trcd_cnt[b] <= trcd_cnt[b] - 1;
            if (trp_cnt[b]  > 0) trp_cnt[b]  <= trp_cnt[b]  - 1;
            if (tras_min[b] > 0) tras_min[b] <= tras_min[b] - 1;
            if (twr_cnt[b]  > 0) twr_cnt[b]  <= twr_cnt[b]  - 1;
            if (trc_cnt[b]  > 0) trc_cnt[b]  <= trc_cnt[b]  - 1;
        end
        if (trfc_cnt > 0) trfc_cnt <= trfc_cnt - 1;
        if (tmrd_cnt > 0) tmrd_cnt <= tmrd_cnt - 1;
        if (trrd_cnt > 0) trrd_cnt <= trrd_cnt - 1;
        // When deselected or NOP, nothing else happens
    end

endmodule
