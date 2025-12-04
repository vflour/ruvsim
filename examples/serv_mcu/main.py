from typing import Any

import re
import os
from bronzebeard.asm import assemble

from ruvsim import PyRunner, PyCompiler

build_dir = "build/sdram_tb"
# Create build directory if it does not exist
if not os.path.exists(build_dir):
    os.makedirs(build_dir)


# Compile the assembly code so that it can be inserted into mem.txt
# for the SERV+SDRAM testbench.
example_dir = os.path.dirname(os.path.abspath(__file__))
machine_code = assemble(path_or_source=os.path.join(example_dir, "serv_mcu.asm"))

# The assembler places 'msg' in .rodata at address 0x140 (see li s0, msg -> 0x01400413),
# but assemble() returns a flat blob without padding. Pad between code and data so that
# the string actually ends up at 0x00000140, otherwise LB reads zeros and the program stalls.
try:
    blob = bytes(machine_code)
    data_marker = b"Hello world!"  # from the source
    idx = blob.find(data_marker)
    if idx != -1 and idx != 0x140:
        # Split code and data and insert zeros up to 0x140
        code_part = blob[:idx]
        data_part = blob[idx:]
        if len(code_part) < 0x140:
            pad_len = 0x140 - len(code_part)
            blob = code_part + (b"\x00" * pad_len) + data_part
        # else: if code already beyond 0x140, leave as-is (unlikely here)
    else:
        blob = blob
except Exception:
    # Fallback to original
    blob = bytes(machine_code)


# split into 4-byte little-endian words for mem.txt
def chunk_words_le(b: bytes):
    # pad to multiple of 4 bytes
    if len(b) % 4 != 0:
        b = b + b"\x00" * (4 - (len(b) % 4))
    for i in range(0, len(b), 4):
        yield b[i : i + 4]


with open(os.path.join(build_dir, "mem.txt"), "wt") as mem_file:
    for w in chunk_words_le(blob):
        mem_file.write(f"{bytes(w)[::-1].hex()}\n")


c = PyCompiler("build/sdram_tb", "", "vlog")
c.enable_system_verilog()
c.set_work("work_SDRAM_tb")
# Compile SDRAM model and the new SERV+SDRAM testbench top
c.add_dependencies(["examples/serv_mcu/serv_sdram_tb.sv"])

serv_source_path = os.path.join(example_dir, "serv")
# Ensure include path for SERV headers such as serv_params.vh
c.add_arg(f"+incdir+{serv_source_path}")
for path in os.listdir(serv_source_path):
    if (path.endswith(".v")) or (path.endswith(".sv")):
        c.add_dependencies([os.path.join(serv_source_path, path)])

c.run()


r = PyRunner(
    "vsim",
    [
        "-work",
        "work_SDRAM_tb",
        "-voptargs=+acc",
        "serv_sdram_tb",
        "-c",
    ],
    "build/sdram_tb",
)
r.wait_until_prompt_startup()
# Run the testbench to completion
r.send_command("log -r /*")
r.run_for(1, "ns")

all_signals = r.get_signals("/serv_sdram_tb/*")

r.run_for(40, "ns")
results = r.send_and_expect_regex('puts "MEMS LIST: [mem list]"', r"^MEMS LIST: .*")
for result in results:
    memory_path_regex: Any = re.compile(r"Verilog: (\/[a-zA-Z_\/]+)")
    memory_path = memory_path_regex.search(result).group(1)
    memory_contents = r.send_and_expect_regex(
        f'puts "MEM DISPLAY: [string map {{\\n ;}} [mem display -format hex -startaddress 0 -endaddress 127 {memory_path} ]]"',
        r"^MEM DISPLAY: .*",
    )
    contents = memory_contents[0].replace("MEM DISPLAY: ", "").split(";")
    # print(contents)


returned = r.send_and_expect_regex(
    "examine /serv_sdram_tb/i_ibus_rdt", r"^# 32'h[xXzZ0-9a-fA-F]+"
)


r.finish()
