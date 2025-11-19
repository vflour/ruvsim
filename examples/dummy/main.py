import os
import time
from ruvsim import PyRunner, PyCompiler

build_dir = "build/dummy"
source_dir = "examples/dummy"
hdl_dir = "examples/hdl"
work_lib = "work_dummy"

if not os.path.exists(build_dir):
    os.makedirs(build_dir)


# Initialize compiler
c = PyCompiler(build_dir, "", "vlog")
c.enable_system_verilog()
c.set_work(work_lib)
c.add_dependencies([os.path.join(hdl_dir, "Dummy.sv"), os.path.join(source_dir, "Dummy_tb.sv")])
c.run()

# Initialize runner
r = PyRunner(
    "vsim",
    [
        "-work",
        work_lib,
        "-voptargs=+acc",
        "Dummy_tb",
        "-c",
    ],
    build_dir,
)
# Then run it to completion
r.wait_until_prompt_startup()
start = time.perf_counter()
r.send_command("log -r /*")
r.run_all()

r.get_log_matches_regex('^Starting testbench')
r.get_log_matches_regex('^# 123')
r.get_log_matches_regex('^a = 10')
r.get_log_matches_regex('^a = 20')

end = time.perf_counter()
print(f"Simulation time: {end - start:.4f} seconds")
print("Dummy_tb ran successfully.")

r.finish()