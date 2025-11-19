module Mem_tb (
    input  logic [5:0]  a,        // 6-bit address for 64 rows
    output logic [7:0]  q         // 8-bit data
);

    // 64 x 8-bit memory
    logic [7:0] mem [0:63];

    // initialize memory with simple values (8-bit = index)
    initial begin
        $readmemh("mem.txt", mem);
        #10;
        q <= mem[a];
        #10;
    end

endmodule