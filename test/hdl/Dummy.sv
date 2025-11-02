module Dummy(
    input wire clk,
    input wire rst,
    input reg [7:0] a
);

    // simple register
    reg [7:0] cnt;

    always @(posedge clk) begin
        if (rst) cnt <= 0;
        else cnt <= cnt + 1;
        cnt <= a + cnt;
    end

endmodule
