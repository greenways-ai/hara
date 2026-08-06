#!/usr/bin/env luajit
-- Benchmark runner for the luajit-hara comparison suite.
-- Contract mirrors rust/src/bin/hara-runtime-benchmark.rs:
--   luajit lua_runner.lua MODE ID SOURCE_HEX EXPECTED WINDOWS CALLS
-- Loads the source once per call (load + call = parse + eval, matching
-- hara's eval_native per-call semantics) and prints one JSON line:
--   {"runtime":"luajit","workload":"ID","first_ns":N,"samples_ns":[...]}

local args = { ... }
if #args ~= 6 then
  io.stderr:write("lua_runner expects MODE ID SOURCE_HEX EXPECTED WINDOWS CALLS\n")
  os.exit(2)
end

local mode = args[1]
local id = args[2]
local source_hex = args[3]
local expected = args[4]
local windows = tonumber(args[5])
local calls = tonumber(args[6])

if not windows or not calls then
  io.stderr:write(id .. ": invalid windows/calls\n")
  os.exit(2)
end

local function decode_hex(value)
  if #value % 2 ~= 0 then return nil, "invalid source hex" end
  local ok, result = pcall(function()
    return (value:gsub("..", function(byte)
      return string.char(tonumber(byte, 16))
    end))
  end)
  if ok then return result end
  return nil, "invalid source hex"
end

local function fail(message)
  io.stderr:write(id .. ": " .. message .. "\n")
  os.exit(1)
end

local function clock_ns()
  return os.clock() * 1e9
end

local source, err = decode_hex(source_hex)
if not source then fail(err) end
local prepared, prepared_err
if mode == "prepared" then prepared, prepared_err = load(source, "workload") end
if mode == "prepared" and not prepared then fail(prepared_err) end

local function eval_once()
  local chunk, load_err = prepared, nil
  if not chunk then chunk, load_err = load(source, "workload") end
  if not chunk then fail(load_err) end
  local ok, value = pcall(chunk)
  if not ok then fail(value) end
  if tostring(value) ~= expected then
    fail("expected " .. expected .. ", got " .. tostring(value))
  end
end

local started = clock_ns()
eval_once()
local first_ns = math.floor(clock_ns() - started + 0.5)

local samples = {}
for _ = 1, windows do
  local window_started = clock_ns()
  for _ = 1, calls do
    eval_once()
  end
  samples[#samples + 1] = math.floor((clock_ns() - window_started) / calls + 0.5)
end

local function json_escape(value)
  return (value:gsub('\\', '\\\\'):gsub('"', '\\"'))
end

io.write('{"runtime":"luajit","workload":"' .. json_escape(id) ..
  '","first_ns":' .. first_ns .. ',"samples_ns":[' ..
  table.concat(samples, ",") .. "]}\n")
