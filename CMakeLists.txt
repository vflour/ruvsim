cmake_minimum_required(VERSION 3.15)

# Project to compile HDL sources with Mentor/Questa 'vlog'
project(rusim_hdl_build LANGUAGES NONE)

# Allow overriding vlog path and flags via -D on the CMake command line
set(VLIB "vlib" CACHE STRING "Path to the 'vlib' executable")
set(VLOG "vlog" CACHE STRING "Path to the 'vlog' executable")
# Prefer a list variable for args so CMake passes them as separate tokens
set(VLOG_ARGS "-sv;-lint;-work;work_SDRAM_tb" CACHE STRING "Args to pass to 'vlog' (CMake list)")

set(CURRENT_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(BUILD_DIR ${CMAKE_CURRENT_BINARY_DIR}/build)

# Collect HDL sources from test/hdl (SystemVerilog and Verilog)
file(GLOB HDL_SOURCES
  "${CURRENT_DIR}/test/hdl/*.sv"
  "${CURRENT_DIR}/test/hdl/*.v"
)

if(NOT HDL_SOURCES)
  message(WARNING "No HDL sources found under ${CURRENT_DIR}/test/hdl")
endif()

# Default target: create build dir, vlib work lib, and invoke vlog with the collected sources
add_custom_target(compile ALL
  COMMAND ${CMAKE_COMMAND} -E make_directory ${BUILD_DIR}
  COMMAND ${CMAKE_COMMAND} -E chdir ${BUILD_DIR} ${VLIB} work_SDRAM_tb
  COMMAND ${CMAKE_COMMAND} -E chdir ${BUILD_DIR} ${VLOG} ${VLOG_ARGS} ${HDL_SOURCES}
  COMMENT "Compiling HDL sources with 'vlog' into ${BUILD_DIR}"
  VERBATIM
)

# Provide an 'install' target so the Rust cmake crate can run `--target install`.
install(CODE "message(STATUS \"Install step (no-op)\")")