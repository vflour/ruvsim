`timescale 1ns/1ns

module serv_sdram_tb(
    output reg clk = 0,
    output reg rst = 1
);
    always #5 clk = ~clk; // 100 MHz

    // Interrupts (none)
    wire i_timer_irq    = 1'b0;

    // -----------------------------
    // SERV core <-> simple Wishbone SRAM
    // -----------------------------
    // Instruction bus
    bit [31:0] o_ibus_adr;
    wire        o_ibus_cyc;
    bit  [31:0] i_ibus_rdt;
    bit         i_ibus_ack;

    // Data bus
    bit [31:0] o_dbus_adr;
    bit [31:0] o_dbus_dat;
    bit  [3:0] o_dbus_sel;
    bit        o_dbus_we;
    bit        o_dbus_cyc;
    bit  [31:0] i_dbus_rdt;
    bit         i_dbus_ack;
    // For readability in waves: SERV keeps o_*bus_adr word-aligned and uses o_dbus_sel for byte lane.
    // Reconstruct the effective byte address for dbus transactions here.
    function automatic [1:0] sel2idx(input [3:0] sel);
        case (sel)
            4'b0001: sel2idx = 2'd0;
            4'b0010: sel2idx = 2'd1;
            4'b0100: sel2idx = 2'd2;
            4'b1000: sel2idx = 2'd3;
            default: sel2idx = 2'd0;
        endcase
    endfunction
    wire [31:0] dbus_eff_adr = {o_dbus_adr[31:2], 2'b00} + {30'd0, sel2idx(o_dbus_sel)};

    // Additional DUT interface wires (unused in this TB)
    wire        o_mdu_valid;
    wire [2:0]  o_ext_funct3;
    wire [31:0] o_ext_rs1;
    wire [31:0] o_ext_rs2;

    // Instantiate SERV core
    serv_rf_top #(
        .RESET_PC(32'h0000_0000)
    ) dut (
        .clk(clk),
        .i_rst(rst),
        // Instruction bus
        .o_ibus_adr(o_ibus_adr),
        .o_ibus_cyc(o_ibus_cyc),
        .i_ibus_rdt(i_ibus_rdt),
        .i_ibus_ack(i_ibus_ack),
        // Data bus
        .o_dbus_adr(o_dbus_adr),
        .o_dbus_dat(o_dbus_dat),
        .o_dbus_sel(o_dbus_sel),
        .o_dbus_we(o_dbus_we),
        .o_dbus_cyc(o_dbus_cyc),
        .i_dbus_rdt(i_dbus_rdt),
        .i_dbus_ack(i_dbus_ack),
        // Interrupts (some variants only implement timer)
        .i_timer_irq(i_timer_irq),
        // External/custom interface tie-offs and MDU valid sink
        .i_ext_ready(1'b0),
        .i_ext_rd(32'h0),
        .o_ext_funct3(o_ext_funct3),
        .o_ext_rs1(o_ext_rs1),
        .o_ext_rs2(o_ext_rs2),
        .o_mdu_valid(o_mdu_valid)
    );

    // Simple unified memory backing both ibus and dbus
    localparam MEM_WORDS = 16384; // 64 KiB
    bit [31:0] mem [0:MEM_WORDS-1];

    // Word address helpers (assuming word-aligned accesses)
    wire [31:0] ibus_word_adr = {o_ibus_adr[31:2], 2'b00};
    wire [31:0] dbus_word_adr = {o_dbus_adr[31:2], 2'b00};

    // Instruction bus: one-shot ack per request with 1-cycle latency
    reg        ibus_req_d;
    reg [31:0] ibus_addr_d;
    reg        ibus_cyc_q;
    always @(posedge clk) begin
        if (rst) begin
            i_ibus_ack  <= 1'b0;
            i_ibus_rdt  <= 32'h0;
            ibus_req_d  <= 1'b0;
            ibus_addr_d <= 32'h0;
            ibus_cyc_q  <= 1'b0;
        end else begin
            i_ibus_ack <= 1'b0; // default
            ibus_cyc_q <= o_ibus_cyc;
            // Latch new request on first cycle of CYC
            if (o_ibus_cyc && !ibus_cyc_q && !ibus_req_d) begin
                ibus_addr_d <= ibus_word_adr;
                ibus_req_d  <= 1'b1;
            end else if (ibus_req_d) begin
                // Respond in the next cycle with data and single ack
                i_ibus_rdt <= mem[ibus_addr_d[17:2]];
                i_ibus_ack <= 1'b1;
                $display("[TB][%0t] IBUS ACK adr=%08x data=%08x", $time, ibus_addr_d, mem[ibus_addr_d[17:2]]);
                ibus_req_d <= 1'b0;
            end
        end
    end

    // Data bus: single-cycle read/write per request with byte enables
    // Note: derive write mask from the latched select to match the request being serviced
    function automatic [31:0] sel_to_mask(input [3:0] sel);
        sel_to_mask = 32'h0;
        for (int i = 0; i < 4; i++) begin
            if (sel[i]) sel_to_mask[i*8 +: 8] = 8'hFF;
        end
    endfunction

    reg        dbus_req_d;
    reg [31:0] dbus_addr_d;
    reg [31:0] dbus_wdata_d;
    reg  [3:0] dbus_sel_d;
    reg        dbus_we_d;
    reg        dbus_cyc_q;
    always @(posedge clk) begin
        if (rst) begin
            i_dbus_ack   <= 1'b0;
            i_dbus_rdt   <= 32'h0;
            dbus_req_d   <= 1'b0;
            dbus_addr_d  <= 32'h0;
            dbus_wdata_d <= 32'h0;
            dbus_sel_d   <= 4'h0;
            dbus_we_d    <= 1'b0;
            dbus_cyc_q   <= 1'b0;
        end else begin
            i_dbus_ack <= 1'b0; // default
            dbus_cyc_q <= o_dbus_cyc;
            // Latch request on first cycle
            if (o_dbus_cyc && !dbus_cyc_q && !dbus_req_d) begin
                dbus_addr_d  <= dbus_word_adr;
                dbus_wdata_d <= o_dbus_dat;
                dbus_sel_d   <= o_dbus_sel;
                dbus_we_d    <= o_dbus_we;
                dbus_req_d   <= 1'b1;
            end else if (dbus_req_d) begin
                // Perform read
                i_dbus_rdt <= mem[dbus_addr_d[17:2]];
                // Perform write if requested
                if (dbus_we_d) begin
                    var automatic [31:0] wmask_local = sel_to_mask(dbus_sel_d);
                    mem[dbus_addr_d[17:2]] <= (dbus_wdata_d & wmask_local) | (mem[dbus_addr_d[17:2]] & ~wmask_local);
                end
                i_dbus_ack <= 1'b1;
                $display("[TB][%0t] DBUS ACK %s adr=%08x eff=%08x sel=%b rdt=%08x wdt=%08x", $time, dbus_we_d?"WR":"RD", dbus_addr_d, {dbus_addr_d[31:2],2'b00}+{30'd0, sel2idx(dbus_sel_d)}, dbus_sel_d, mem[dbus_addr_d[17:2]], dbus_wdata_d);
                dbus_req_d <= 1'b0;
            end
        end
    end

    // Load program/data image if available
    initial begin
        if ($test$plusargs("NOLOAD")) begin
            $display("[TB] Skipping memory load (NOLOAD)");
        end else begin
            $display("[TB] Loading mem.txt if present...");
            // Load from repository path relative to sim cwd (build/sdram_tb)
            $readmemh("./mem.txt", mem);
        end
    end

    // -----------------------------
    // Reset sequence and run control
    // -----------------------------
    initial begin
        repeat (10) @(posedge clk);
        rst = 0;
        $display("[TB] serv_sdram_tb started. Running for 2 ms or until $finish");
        repeat (1000_000) @(posedge clk); // 2 ms at 100 MHz
        $display("[TB] Timeout reached. Finishing.");
        $finish;
    end
    
endmodule
