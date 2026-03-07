---
triggers: ["Erlang", "OTP erlang", "cowboy", "erlang gen_server", "erlang supervisor", "mochiweb", "erlang distribution"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["erl"]
category: erlang
---

# Erlang/OTP and Cowboy

When working with Erlang/OTP:

1. Structure applications as OTP apps with `src/`, `include/`, and `priv/` directories; define the `.app.src` file with `mod`, `applications`, and `registered` keys — use `rebar3` as the build tool with `rebar3 new app myapp` to scaffold.
2. Implement `gen_server` for stateful processes — export `start_link/1`, define `init/1`, `handle_call/3`, `handle_cast/2`, and `handle_info/2` callbacks; always handle unexpected messages in `handle_info` to avoid mailbox buildup.
3. Design supervision trees using `supervisor` behaviour — use `one_for_one` for independent workers, `simple_one_for_one` (or `simple_one_for_one` replaced by dynamic supervisors in newer OTP) for pools; set `max_restarts` and `max_seconds` to prevent restart storms.
4. Use pattern matching in function heads and case expressions for control flow — match on tuples like `{ok, Value}` and `{error, Reason}` as the idiomatic way to handle success and failure across function boundaries.
5. For HTTP services with Cowboy, define routes in `cowboy_router:compile/1` and start listeners with `cowboy:start_clear/3` or `cowboy:start_tls/3`; implement handlers with `init/2` returning `{ok, Req, State}` and use `cowboy_req` functions for request/response.
6. Use ETS tables for shared in-memory state — create with `ets:new(Name, [set, public, {read_concurrency, true}])` for read-heavy loads; prefer `ets:insert/2` and `ets:lookup/2` over gen_server state for high-throughput data access.
7. Write tests with Common Test (`-include_lib("common_test/include/ct.hrl")`) for integration tests and EUnit (`-include_lib("eunit/include/eunit.hrl")`) for unit tests; run with `rebar3 ct` and `rebar3 eunit` respectively.
8. Handle distributed Erlang by starting nodes with `-name` or `-sname` flags and connecting with `net_adm:ping/1`; use `pg` (process groups) or `global` for process registration across nodes — secure distribution with `-setcookie` and firewall EPMD port 4369.
9. Use `gen_statem` for stateful protocols and workflows — define states as callback functions (`state_name/3`) or use `handle_event_function` mode for a single handler; this replaces the deprecated `gen_fsm` and handles complex state transitions cleanly.
10. Manage application configuration in `sys.config` and `vm.args` for releases; build releases with `rebar3 release` using `relx` configuration in `rebar.config` — this produces a self-contained directory with ERTS for deployment.
11. Use `proc_lib` and `sys` modules for custom special processes when `gen_server` is too heavy — implement `system_continue/3`, `system_terminate/4`, and `system_code_change/4` for OTP compliance with `sys:get_status/1` support.
12. Debug production systems with `observer:start()` for GUI monitoring, `recon` library for safe production introspection, `dbg` or `recon_trace` for function tracing, and `crashdump_viewer` for post-mortem analysis of `erl_crash.dump` files.
