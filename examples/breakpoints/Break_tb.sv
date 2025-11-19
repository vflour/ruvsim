`timescale 1ns/1ps
module Break_tb;
    reg clk = 0;
    reg rst = 0;
    reg [7:0] counter = 0;

    always #5 clk = ~clk;

    initial begin
        rst = 1;
        #12 rst = 0;
        repeat (20) begin
            #10 counter = counter + 1;
        end
        $display("Done counter=%0d", counter);
        $finish;
    end
endmodule
