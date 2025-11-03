RuvSim REST API

Overview
- Axum-based REST API to compile HDL deps, start a ModelSim/Questa session, inspect nets, run simulation steps, and query logs.
- OpenAPI docs served at /docs and /api-docs/openapi.json.

Environment
- RUVSIM_API_ADDR: Bind address (default 0.0.0.0:8080)
- RUVSIM_WORK_DIR: Required. Base directory for compilation and simulator CWD.
- RUVSIM_MODELSIM_PATH: Optional. Path to ModelSim/Questa bin folder for vlib/vlog/vcom.
- RUVSIM_VSIM_BIN: Optional. Simulator executable name (default "vsim").
- RUVSIM_SESSION_TTL_SECS: Optional. Idle eviction TTL in seconds (default 600).

Path policy
- The API rejects absolute dependency paths.
- deps must be relative to RUVSIM_WORK_DIR. The server resolves each dep as "${RUVSIM_WORK_DIR}/${dep}".

Key endpoints
- GET /health
- GET /sessions
- POST /sessions
  Body: { "work_lib": "work_SDRAM_tb", "top": "SDRAM_tb", "deps": ["hdl/SDRAM_tb.sv", "hdl/SDRAM.sv", "hdl/Dummy.sv", "hdl/Dummy_tb.sv"] }
  Returns: { "id": "<uuid>" }
- GET /sessions/{id}/nets?path=/*&direction=All|Input|Output|Inout&examine=true&radix=binary
- POST /sessions/{id}/run  Body: { "mode": "all" } | { "mode": "next" } | { "mode": "for", "ns": 100 }
- GET /sessions/{id}/logs?contains=NETS
- POST /sessions/{id}/examine  Body: { "path": "/top/u0/signal", "radix": "hex" }
- POST /sessions/{id}/cmd  Body: { "command": "run -next", "expect_contains": "VSIM" }
- DELETE /sessions/{id}

Notes
- Compilation uses vlog (SystemVerilog enabled). vcom is not exposed yet.
- work library is created under RUVSIM_WORK_DIR if not present.
- Errors are returned as { "message": "..." } with suitable HTTP status codes.