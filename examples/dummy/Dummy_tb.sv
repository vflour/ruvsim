`timescale 1ns/1ps
module Dummy_tb(
    output reg clk,
    output reg rst,
    output reg [7:0] a,
    output wire b
);

    // instantiate DUT
    Dummy dut(.clk(clk), .rst(rst), .a(a));

    assign b = a[0];

    always #5 clk = ~clk; // 100 MHz-ish for simulation speed

    reg [15:0][0:3] memory_slot [0:3];

    initial begin
        $readmemh("mem.txt", memory_slot);
        a = 0;
        clk = 0;
        rst = 1;
        
        $display("Starting testbench");
        a = a + 10;
        #20 rst = 0;
        // a should be 10
        $display("a = %d", a);
        a = a + 10;
        #100 $display("# 123");
        // a should be 20
        $display("a = %d", a);
        $finish;
    end
endmodule
