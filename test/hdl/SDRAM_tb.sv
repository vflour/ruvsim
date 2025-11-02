`timescale 1ns/1ns
module SDRAM_tb(
    // Expose a few signals as outputs for observability
    output reg        clk,
    output reg        cke,
    output reg        cs_n,
    output reg        ras_n,
    output reg        cas_n,
    output reg        we_n,
    output reg  [1:0] ba,
    output reg  [12:0] addr,
    output reg  [3:0] dqm,
    inout  wire [31:0] dq
);

    // Instantiate DUT (functional SDRAM model)
    SDRAM dut(
        .clk(clk), .cke(cke), .cs_n(cs_n), .ras_n(ras_n), .cas_n(cas_n), .we_n(we_n),
        .ba(ba), .addr(addr), .dq(dq), .dqm(dqm)
    );

    // 200 MHz clock (5ns period)
    always #2.5 clk = ~clk;

    // Drive for DQ (tri-state when not writing)
    reg         dq_drive_en;
    reg [31:0]  dq_drive_data;
    assign dq = dq_drive_en ? dq_drive_data : 32'hzzzz_zzzz;

    // Helpers for sampling read data
    reg [31:0] last_read;
    // Predeclare read data buffers at module scope to satisfy tools that require declarations before statements
    reg [31:0] rddata  [0:3];
    reg [31:0] rddata2 [0:3];
    reg [31:0] rddata3 [0:3];
    reg [31:0] rd8_seq [0:7];
    reg [31:0] rd8_int [0:7];
    reg [31:0] rd2     [0:1];
    // Helper variable for timing-violation baseline
    integer base_count;

    // Command issue helper: apply signals prior to posedge
    task automatic issue_cmd(
        input bit cs, input bit ras, input bit cas, input bit we,
        input [1:0] bank, input [12:0] a
    );
        begin
            @(negedge clk);
            cs_n  <= cs;
            ras_n <= ras;
            cas_n <= cas;
            we_n  <= we;
            ba    <= bank;
            addr  <= a;
            @(posedge clk); // command sampled on this edge
            // Return to NOP on the following half-cycle to avoid repeating the command
            @(negedge clk);
            cs_n  <= 1'b1;
            ras_n <= 1'b1;
            cas_n <= 1'b1;
            we_n  <= 1'b1;
        end
    endtask

    // Specific command wrappers (active low inputs)
    task automatic cmd_nop();            issue_cmd(0, 1, 1, 1, 2'b00, 13'd0); endtask
    task automatic cmd_precharge_all();  issue_cmd(0, 0, 1, 0, 2'b00, 13'b1_0000_0000_0000); endtask // A10=1
    task automatic cmd_precharge(input [1:0] bank); issue_cmd(0,0,1,0,bank,13'b0); endtask
    task automatic cmd_refresh();        issue_cmd(0, 0, 0, 1, 2'b00, 13'd0); endtask
    task automatic cmd_active(input [1:0] bank, input [12:0] row);
        issue_cmd(0, 0, 1, 1, bank, row);
    endtask
    task automatic cmd_load_mode(
        input [2:0] BL, input BT, input [2:0] CL, input [2:0] OM, input WB
    );
        // Mode register mapping used by the SDRAM model
        // addr[2:0]=BL, addr[3]=BT, addr[6:4]=CL, addr[9:7]=OM, addr[9]=WB (approx)
        reg [12:0] m;
        begin
            m = 13'd0;
            m[2:0] = BL;
            m[3]   = BT;
            m[6:4] = CL;
            m[9:7] = OM;
            m[9]   = WB; // kept for compatibility with model
            issue_cmd(0, 0, 0, 0, 2'b00, m);
        end
    endtask
    task automatic cmd_read(input [1:0] bank, input [12:0] col, input bit autopre);
        reg [12:0] a;
        begin a = col; a[10] = autopre; issue_cmd(0, 1, 0, 1, bank, a); end
    endtask
    task automatic cmd_write(input [1:0] bank, input [12:0] col, input bit autopre);
        reg [12:0] a;
        begin a = col; a[10] = autopre; issue_cmd(0, 1, 0, 0, bank, a); end
    endtask

    // Drive a write burst: present first beat for the immediate next posedge after WRITE
    task automatic drive_write_burst(input int beats, input [31:0] base);
        int i;
        begin
            // We return from cmd_write at a negedge; drive D0 now for capture on the next posedge
            dq_drive_en = 1'b1;
            dqm         = 4'b0000; // no mask
            dq_drive_data = base + 0;
            @(posedge clk); // DUT captures first beat here
            for (i = 1; i < beats; i = i + 1) begin
                @(negedge clk);
                dq_drive_data = base + i;
                @(posedge clk); // DUT captures subsequent beats here
            end
            @(negedge clk);
            dq_drive_en = 1'b0;
        end
    endtask

    // Drive a masked write burst (constant dqm), first beat for immediate next posedge
    task automatic drive_write_burst_masked(input int beats, input [31:0] base, input [3:0] mask);
        int i;
        begin
            dq_drive_en = 1'b1;
            dqm         = mask; // 1 = mask that byte
            dq_drive_data = base + 0;
            @(posedge clk);
            for (i = 1; i < beats; i = i + 1) begin
                @(negedge clk);
                dq_drive_data = base + i;
                @(posedge clk);
            end
            @(negedge clk);
            dq_drive_en = 1'b0;
            dqm         = 4'b0000;
        end
    endtask

    // Read and collect exactly 4 beats with CAS latency cycles wait
    task automatic collect_read_burst4(input int cl, output logic [31:0] data[0:3]);
        int i;
        begin
            // Model drives first beat CL cycles after READ: wait CL-1 cycles
            if (cl > 0) repeat (cl-1) @(posedge clk);
            for (i = 0; i < 4; i = i + 1) begin
                @(posedge clk);       // beat i driven here
                @(negedge clk);       // sample mid-cycle
                last_read = dq;
                data[i] = last_read;
            end
        end
    endtask

    // Read and collect exactly 2 beats with CAS latency cycles wait
    task automatic collect_read_burst2(input int cl, output logic [31:0] data[0:1]);
        int i;
        begin
            if (cl > 0) repeat (cl-1) @(posedge clk);
            for (i = 0; i < 2; i = i + 1) begin
                @(posedge clk);
                @(negedge clk);
                data[i] = dq;
            end
        end
    endtask

    // Helper: expect a violation code increment in the DUT, given a baseline count
    task automatic expect_violation(input int expected_code, input string name, input int count_before);
        begin
            // allow 1-2 cycles for violation accounting
            @(posedge clk); @(posedge clk);
            if (dut.violation_count != count_before + 1) begin
                $fatal(1, "[%s] Expected violation_count to increment (before=%0d, after=%0d)", name, count_before, dut.violation_count);
            end
            if (dut.last_violation_code != expected_code) begin
                $fatal(1, "[%s] Expected last_violation_code=%0d, got %0d", name, expected_code, dut.last_violation_code);
            end
            $display("[TB] Violation '%s' detected as expected (code %0d)", name, expected_code);
        end
    endtask

    // Variant: accept either of two codes
    task automatic expect_violation_one_of(input int code_a, input int code_b, input string name, input int count_before);
        begin
            @(posedge clk); @(posedge clk);
            if (dut.violation_count != count_before + 1) begin
                $fatal(1, "[%s] Expected violation_count to increment (before=%0d, after=%0d)", name, count_before, dut.violation_count);
            end
            if (!(dut.last_violation_code == code_a || dut.last_violation_code == code_b)) begin
                $fatal(1, "[%s] Expected last_violation_code in {%0d,%0d}, got %0d", name, code_a, code_b, dut.last_violation_code);
            end
            $display("[TB] Violation '%s' detected as expected (code %0d)", name, dut.last_violation_code);
        end
    endtask

    // Helper: before first data, ensure dq is Z for CL-1 cycles
    task automatic wait_for_first_data(input int cl);
        int i;
        begin
            if (cl > 1) begin
                for (i = 0; i < cl-1; i = i + 1) begin
                    @(posedge clk);
                    @(negedge clk);
                    if (dq !== 32'hzzzz_zzzz) begin
                        $fatal(1, "[CL] dq should be Z before first data (cycle %0d)", i);
                    end
                end
            end
        end
    endtask

    // Collect exactly 8-beat read burst without additional CL waiting
    task automatic collect_read_burst8_no_wait(output logic [31:0] data[0:7]);
        int i;
        begin
            for (i = 0; i < 8; i = i + 1) begin
                @(posedge clk);
                @(negedge clk);
                data[i] = dq;
            end
        end
    endtask

    // Test sequence
    initial begin
        // Init defaults
        clk = 0;
        cke = 0; cs_n = 1; ras_n = 1; cas_n = 1; we_n = 1;
        ba = 0; addr = 0; dqm = 4'b0000;
        dq_drive_en = 0; dq_drive_data = 32'h0;

        $display("[TB] Power-up and initialization");
        // Power-up: hold CKE low for a few cycles
        repeat (5) @(posedge clk);
        cke <= 1;

        // NOP after CKE high
        cmd_nop();
        // Precharge all
        cmd_precharge_all();
        // 2x Auto-Refresh
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        // Load mode: BL=4, BT=sequential, CL=2, OM=0, WB=0
        cmd_load_mode(3'd2, 1'b0, 3'd2, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();

        // Activate bank 0, row 0x012
        $display("[TB] ACTIVATE bank 0 row 0x012");
        cmd_active(2'd0, 13'h012);
        cmd_nop(); cmd_nop(); // tRCD spacing (rough)

        // WRITE burst of 4 beats at column 0x01
        $display("[TB] WRITE burst BL=4 @ col 0x01");
        cmd_write(2'd0, 13'h001, 1'b0);
        drive_write_burst(4, 32'hA0A0_A000);
        cmd_nop();

        // READ burst from same address
        $display("[TB] READ burst BL=4 @ col 0x01");
        cmd_read(2'd0, 13'h001, 1'b0);
        collect_read_burst4(2, rddata);
        $display("[TB] Read data: %08X %08X %08X %08X", rddata[0], rddata[1], rddata[2], rddata[3]);

        // Check data matches written pattern
        if (rddata[0] !== 32'hA0A0_A000 || rddata[1] !== 32'hA0A0_A001 ||
            rddata[2] !== 32'hA0A0_A002 || rddata[3] !== 32'hA0A0_A003) begin
            $fatal(1, "Readback mismatch after WRITE/READ burst");
        end

        // Masked write test (mask byte0)
        $display("[TB] Masked WRITE (mask byte0) then READ");
        cmd_write(2'd0, 13'h010, 1'b0);
        drive_write_burst_masked(4, 32'hDEAD_BEE0, 4'b0001);
        cmd_nop();
        cmd_read(2'd0, 13'h010, 1'b0);
        collect_read_burst4(2, rddata2);
        // Expect byte0 preserved from previous content; since uninitialized, just check mask applied per-beat not Xs
        // We validate that masked byte is not equal to driven low nibble sequence when mask active
        if ((rddata2[0] & 32'h0000_00FF) === (32'hDEAD_BEE0 & 32'h0000_00FF)) begin
            $fatal(1, "DQM mask did not take effect on byte0");
        end

        // Auto-precharge on READ
        $display("[TB] READ with auto-precharge");
        cmd_active(2'd1, 13'h100);
        cmd_nop(); cmd_nop();
        cmd_read(2'd1, 13'h020, 1'b1); // autoprecharge
        collect_read_burst4(2, rddata3);
        cmd_nop(); // allow precharge to complete

        // ------------------------------------------------------------
        // Timing violation tests
        // ------------------------------------------------------------
        $display("[TB] Begin timing violation tests");

        // tRCD (ACT->RD too soon)
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_active(2'd2, 13'h010);
        base_count = dut.violation_count;
        cmd_read(2'd2, 13'h000, 1'b0); // too soon, tRCD_CYC=2
        expect_violation(2, "tRCD_RD", base_count);

        // tRCD (ACT->WR too soon)
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_active(2'd2, 13'h011);
        base_count = dut.violation_count;
        cmd_write(2'd2, 13'h000, 1'b0); // too soon
        // (we won't drive data because WR is rejected)
        expect_violation(1, "tRCD_WR", base_count);

        // tWR (WR->PRE too soon): valid ACT, wait tRCD, do WRITE, then PRE immediately
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h020);
        cmd_nop(); cmd_nop(); // satisfy tRCD=2
        cmd_write(2'd0, 13'h000, 1'b0);
        drive_write_burst(1, 32'hBBBB_BBBB);
        base_count = dut.violation_count;
        cmd_precharge(2'd0); // too soon after WR (tWR_CYC=2)
        expect_violation(3, "tWR", base_count);

        // tRP (PRE->ACT too soon): ensure PRE actually occurs on an active bank
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h030);
        cmd_nop(); cmd_nop(); // satisfy tRAS min before PRE
        cmd_precharge(2'd0);   // start tRP window
        base_count = dut.violation_count;
        cmd_active(2'd0, 13'h031); // too soon, violate tRP=2
        expect_violation(4, "tRP", base_count);
        // recover
        cmd_nop(); cmd_nop();

        // tRAS (ACT->PRE too soon)
        cmd_active(2'd1, 13'h040);
        base_count = dut.violation_count;
        cmd_precharge(2'd1); // too early (tRAS_MIN=2)
        expect_violation(7, "tRAS", base_count);
        cmd_nop(); cmd_nop();

        // tMRD (MRS recovery): MRS then immediate ACT should be blocked by global window
        cmd_precharge_all(); cmd_nop();
        cmd_load_mode(3'd2, 1'b0, 3'd2, 3'd0, 1'b0);
        base_count = dut.violation_count;
        cmd_active(2'd3, 13'h001); // issued during tMRD -> global block
        expect_violation(8, "tMRD_global_block", base_count);
        // recover tMRD window
        cmd_nop(); cmd_nop();

        // MRS requires all banks precharged: attempt MRS while a bank is active
        cmd_active(2'd3, 13'h002);
        base_count = dut.violation_count;
        cmd_load_mode(3'd2, 1'b0, 3'd2, 3'd0, 1'b0); // should flag requirement violation
        expect_violation(6, "MRS_requires_precharge", base_count);
        // clean up
        cmd_precharge(2'd3); cmd_nop(); cmd_nop();

        // tRRD (ACT->ACT too soon across banks)
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h050);
        base_count = dut.violation_count;
        cmd_active(2'd1, 13'h060); // too soon, violates tRRD=2 cycles
        expect_violation(9, "tRRD", base_count);

        // tRC (row cycle: ACT->ACT same bank too soon, even if PRE satisfied)
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h070);
        // wait tRAS min (2 cycles in model)
        cmd_nop(); cmd_nop();
        cmd_precharge(2'd0);
        // issue ACT immediately to maximize chance of tRC violation (may also flag tRP)
        base_count = dut.violation_count;
        cmd_active(2'd0, 13'h071); // violates tRC = tRAS + tRP + 1
        expect_violation_one_of(10, 4, "tRC", base_count);

        // tRFC (issue ACT during refresh recovery window)
        cmd_precharge_all(); cmd_nop();
        cmd_refresh();
        base_count = dut.violation_count;
        cmd_active(2'd2, 13'h080); // blocked by global tRFC window
        expect_violation(8, "tRFC_global_block", base_count);

        // ------------------------------------------------------------
        // Mode variation tests: CL=3, BL=8, BT=0/1, WB=1
        // ------------------------------------------------------------
        // CL=3, BL=8 sequential burst
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        // BL=8 (code 3), BT=0, CL=3 (code 3), OM=0, WB=0
        cmd_load_mode(3'd3, 1'b0, 3'd3, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h200);
        cmd_nop(); cmd_nop(); // tRCD
        cmd_write(2'd0, 13'h020, 1'b0);
        drive_write_burst(8, 32'hC300_0000);
        cmd_nop();
        cmd_read(2'd0, 13'h020, 1'b0);
        wait_for_first_data(3);
        collect_read_burst8_no_wait(rd8_seq);
        if (rd8_seq[0] !== 32'hC300_0000 || rd8_seq[1] !== 32'hC300_0001 ||
            rd8_seq[2] !== 32'hC300_0002 || rd8_seq[3] !== 32'hC300_0003 ||
            rd8_seq[4] !== 32'hC300_0004 || rd8_seq[5] !== 32'hC300_0005 ||
            rd8_seq[6] !== 32'hC300_0006 || rd8_seq[7] !== 32'hC300_0007) begin
            $fatal(1, "BL=8 sequential readback mismatch (CL=3)");
        end

        // BL=8, BT=1 interleaved burst (write and read use same interleaved order)
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd3, 1'b1, 3'd3, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h210);
        cmd_nop(); cmd_nop();
        cmd_write(2'd0, 13'h030, 1'b0);
        drive_write_burst(8, 32'hD300_0000);
        cmd_nop();
        cmd_read(2'd0, 13'h030, 1'b0);
        wait_for_first_data(3);
        collect_read_burst8_no_wait(rd8_int);
        if (rd8_int[0] !== 32'hD300_0000 || rd8_int[1] !== 32'hD300_0001 ||
            rd8_int[2] !== 32'hD300_0002 || rd8_int[3] !== 32'hD300_0003 ||
            rd8_int[4] !== 32'hD300_0004 || rd8_int[5] !== 32'hD300_0005 ||
            rd8_int[6] !== 32'hD300_0006 || rd8_int[7] !== 32'hD300_0007) begin
            $fatal(1, "BL=8 interleaved readback mismatch (CL=3)");
        end

        // WB=1 forces single write even when BL=8
        // Prefill 8 locations with a baseline pattern using WB=0
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd3, 1'b0, 3'd3, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h220);
        cmd_nop(); cmd_nop();
        cmd_write(2'd0, 13'h040, 1'b0);
        drive_write_burst(8, 32'hE300_0000);
        // satisfy tWR then precharge for MRS
        cmd_nop(); cmd_nop();
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        // Now set WB=1
        cmd_load_mode(3'd3, 1'b0, 3'd3, 3'd0, 1'b1);
        cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h220);
        cmd_nop(); cmd_nop();
        cmd_write(2'd0, 13'h040, 1'b0);
        // Attempt to drive 8 beats; model should capture only first beat when WB=1
        drive_write_burst(8, 32'hF300_0000);
        cmd_nop();
        // Read back 8 locations
        cmd_read(2'd0, 13'h040, 1'b0);
        wait_for_first_data(3);
        collect_read_burst8_no_wait(rd8_seq);
        if (rd8_seq[0] !== 32'hF300_0000 || rd8_seq[1] !== 32'hE300_0001 ||
            rd8_seq[2] !== 32'hE300_0002 || rd8_seq[3] !== 32'hE300_0003 ||
            rd8_seq[4] !== 32'hE300_0004 || rd8_seq[5] !== 32'hE300_0005 ||
            rd8_seq[6] !== 32'hE300_0006 || rd8_seq[7] !== 32'hE300_0007) begin
            $fatal(1, "WB=1 single-write behavior mismatch");
        end

        // ------------------------------------------------------------
        // Additional coverage: CL=2 with BL=2 and BL=4 (sequential)
        // ------------------------------------------------------------
        // CL=2, BL=2
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd1, 1'b0, 3'd2, 3'd0, 1'b0); // BL=2, BT=0, CL=2
        cmd_nop(); cmd_nop();
        cmd_active(2'd2, 13'h300);
        cmd_nop(); cmd_nop();
        cmd_write(2'd2, 13'h010, 1'b0);
        drive_write_burst(2, 32'hC200_0000);
        cmd_nop();
        cmd_read(2'd2, 13'h010, 1'b0);
        collect_read_burst2(2, rd2);
        if (rd2[0] !== 32'hC200_0000 || rd2[1] !== 32'hC200_0001) begin
            $fatal(1, "CL=2 BL=2 sequential readback mismatch");
        end

        // CL=2, BL=4
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd2, 1'b0, 3'd2, 3'd0, 1'b0); // BL=4, BT=0, CL=2
        cmd_nop(); cmd_nop();
        cmd_active(2'd2, 13'h320);
        cmd_nop(); cmd_nop();
        cmd_write(2'd2, 13'h014, 1'b0);
        drive_write_burst(4, 32'hC240_0000);
        cmd_nop();
        cmd_read(2'd2, 13'h014, 1'b0);
        collect_read_burst4(2, rddata);
        if (rddata[0] !== 32'hC240_0000 || rddata[1] !== 32'hC240_0001 ||
            rddata[2] !== 32'hC240_0002 || rddata[3] !== 32'hC240_0003) begin
            $fatal(1, "CL=2 BL=4 sequential readback mismatch");
        end

        // CL=2, BL=2, BT=1 (interleaved)
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd1, 1'b1, 3'd2, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd3, 13'h340);
        cmd_nop(); cmd_nop();
        cmd_write(2'd3, 13'h018, 1'b0);
        drive_write_burst(2, 32'hC2B2_0000);
        cmd_nop();
        cmd_read(2'd3, 13'h018, 1'b0);
        collect_read_burst2(2, rd2);
        if (rd2[0] !== 32'hC2B2_0000 || rd2[1] !== 32'hC2B2_0001) begin
            $fatal(1, "CL=2 BL=2 interleaved readback mismatch");
        end

        // CL=2, BL=4, BT=1 (interleaved)
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd2, 1'b1, 3'd2, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd3, 13'h360);
        cmd_nop(); cmd_nop();
        cmd_write(2'd3, 13'h01C, 1'b0);
        drive_write_burst(4, 32'hC2B4_0000);
        cmd_nop();
        cmd_read(2'd3, 13'h01C, 1'b0);
        collect_read_burst4(2, rddata);
        if (rddata[0] !== 32'hC2B4_0000 || rddata[1] !== 32'hC2B4_0001 ||
            rddata[2] !== 32'hC2B4_0002 || rddata[3] !== 32'hC2B4_0003) begin
            $fatal(1, "CL=2 BL=4 interleaved readback mismatch");
        end

        // CL=3, BL=2, sequential
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd1, 1'b0, 3'd3, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd1, 13'h3A0);
        cmd_nop(); cmd_nop();
        cmd_write(2'd1, 13'h008, 1'b0);
        drive_write_burst(2, 32'hC320_0000);
        cmd_nop();
        cmd_read(2'd1, 13'h008, 1'b0);
        collect_read_burst2(3, rd2);
        if (rd2[0] !== 32'hC320_0000 || rd2[1] !== 32'hC320_0001) begin
            $fatal(1, "CL=3 BL=2 sequential readback mismatch");
        end

        // CL=3, BL=4, sequential
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd2, 1'b0, 3'd3, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd1, 13'h3C0);
        cmd_nop(); cmd_nop();
        cmd_write(2'd1, 13'h00C, 1'b0);
        drive_write_burst(4, 32'hC340_0000);
        cmd_nop();
        cmd_read(2'd1, 13'h00C, 1'b0);
        collect_read_burst4(3, rddata);
        if (rddata[0] !== 32'hC340_0000 || rddata[1] !== 32'hC340_0001 ||
            rddata[2] !== 32'hC340_0002 || rddata[3] !== 32'hC340_0003) begin
            $fatal(1, "CL=3 BL=4 sequential readback mismatch");
        end

        // CL=3, BL=2, BT=1 interleaved
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_refresh(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd1, 1'b1, 3'd3, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd1, 13'h3E0);
        cmd_nop(); cmd_nop();
        cmd_write(2'd1, 13'h004, 1'b0);
        drive_write_burst(2, 32'hC3B2_0000);
        cmd_nop();
        cmd_read(2'd1, 13'h004, 1'b0);
        collect_read_burst2(3, rd2);
        if (rd2[0] !== 32'hC3B2_0000 || rd2[1] !== 32'hC3B2_0001) begin
            $fatal(1, "CL=3 BL=2 interleaved readback mismatch");
        end

        // CL=3, BL=4, BT=1 interleaved
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd2, 1'b1, 3'd3, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd1, 13'h3F0);
        cmd_nop(); cmd_nop();
        cmd_write(2'd1, 13'h006, 1'b0);
        drive_write_burst(4, 32'hC3B4_0000);
        cmd_nop();
        cmd_read(2'd1, 13'h006, 1'b0);
        collect_read_burst4(3, rddata);
        if (rddata[0] !== 32'hC3B4_0000 || rddata[1] !== 32'hC3B4_0001 ||
            rddata[2] !== 32'hC3B4_0002 || rddata[3] !== 32'hC3B4_0003) begin
            $fatal(1, "CL=3 BL=4 interleaved readback mismatch");
        end

        // DQM mask tests under CL=2 and CL=3
        // CL=2, BL=4, mask low 16 bits (dqm=0011), expect lower bytes preserved from baseline
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd2, 1'b0, 3'd2, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h400);
        cmd_nop(); cmd_nop();
        // Baseline fill
        cmd_write(2'd0, 13'h020, 1'b0);
        drive_write_burst(4, 32'hD240_1000);
        cmd_nop();
        // Masked overwrite
        cmd_write(2'd0, 13'h020, 1'b0);
        drive_write_burst_masked(4, 32'hD240_2000, 4'b0011);
        cmd_nop();
        cmd_read(2'd0, 13'h020, 1'b0);
        collect_read_burst4(2, rddata);
        // Check per beat
        if (rddata[0][31:16] !== 16'hD240 && rddata[0][15:0] !== 16'h1000) $fatal(1, "CL=2 DQM(0011) beat0 mismatch");
        if (rddata[1][31:16] !== 16'hD241 && rddata[1][15:0] !== 16'h1001) $fatal(1, "CL=2 DQM(0011) beat1 mismatch");
        if (rddata[2][31:16] !== 16'hD242 && rddata[2][15:0] !== 16'h1002) $fatal(1, "CL=2 DQM(0011) beat2 mismatch");
        if (rddata[3][31:16] !== 16'hD243 && rddata[3][15:0] !== 16'h1003) $fatal(1, "CL=2 DQM(0011) beat3 mismatch");

        // CL=3, BL=2, mask high 16 bits (dqm=1100), expect upper bytes preserved from baseline
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd1, 1'b0, 3'd3, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd0, 13'h420);
        cmd_nop(); cmd_nop();
        // Baseline fill
        cmd_write(2'd0, 13'h028, 1'b0);
        drive_write_burst(2, 32'hD320_3000);
        cmd_nop();
        // Masked overwrite
        cmd_write(2'd0, 13'h028, 1'b0);
        drive_write_burst_masked(2, 32'hD320_4000, 4'b1100);
        cmd_nop();
        cmd_read(2'd0, 13'h028, 1'b0);
        collect_read_burst2(3, rd2);
        if (rd2[0][31:16] !== 16'hD320 || rd2[0][15:0] !== 16'h4000) $fatal(1, "CL=3 DQM(1100) beat0 mismatch");
        if (rd2[1][31:16] !== 16'hD320 || rd2[1][15:0] !== 16'h4001) $fatal(1, "CL=3 DQM(1100) beat1 mismatch");

        // Auto-precharge confirmation: provoke tRP after READ with A10=1
        // CL=2 case
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd2, 1'b0, 3'd2, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd2, 13'h440);
        cmd_nop(); cmd_nop();
        cmd_read(2'd2, 13'h030, 1'b1); // auto-precharge
        collect_read_burst4(2, rddata);
        base_count = dut.violation_count;
        cmd_active(2'd2, 13'h441); // should hit tRP if PRE occurred
        expect_violation(4, "CL2_READ_AP_tRP", base_count);

        // CL=3 case
        cmd_precharge_all(); cmd_nop(); cmd_nop();
        cmd_load_mode(3'd1, 1'b0, 3'd3, 3'd0, 1'b0);
        cmd_nop(); cmd_nop();
        cmd_active(2'd2, 13'h460);
        cmd_nop(); cmd_nop();
        cmd_read(2'd2, 13'h032, 1'b1);
        collect_read_burst2(3, rd2);
        base_count = dut.violation_count;
        cmd_active(2'd2, 13'h461);
        expect_violation(4, "CL3_READ_AP_tRP", base_count);


        $display("[TB] Timing violation tests completed");
        $display("[TB] SDRAM_tb completed successfully");
        #20 $finish;
    end
endmodule
