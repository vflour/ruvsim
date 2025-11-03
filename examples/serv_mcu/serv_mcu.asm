li s0, msg

loop:
    lb t1, 0(s0)
    beq t1, zero, done
    addi s0, s0, 1
    j loop

done:
    li t2, 1
    j done

msg: 
    string Hello world!
    db 0
align 4
