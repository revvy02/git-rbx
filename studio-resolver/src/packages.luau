--[[
	Package loading without hand vendoring: rojo builds roblox_packages into
	an rbxm (host-side, via @rodeo/process), roblox.import turns it into real
	instances, and the pesde instance-path shims — which don't survive
	single-file bundling — resolve natively from there. Only string requires
	go through the bundler; require(<ModuleScript>) happens at runtime.

	The resolver checkout root comes from --resolver-root (the rbx-diff
	launcher passes it); running by hand from studio-resolver, cwd is already
	the root.
]]

local ServerStorage = game:GetService("ServerStorage")

local fs = require("@rodeo/fs")
local process = require("@rodeo/process")
local roblox = require("@rodeo/roblox")

local root = "."
for index, arg in process.args do
	if arg == "--resolver-root" then
		root = process.args[index + 1]
	end
end

if not fs.exists(`{root}/build`) then
	fs.mkdir(`{root}/build`)
end
local result = process.run({ "rojo", "build", `{root}/packages.project.json`, "-o", `{root}/build/packages.rbxm` })
assert(result.ok, `rojo build of roblox_packages failed: {result.err}`)

local stale = ServerStorage:FindFirstChild("GitRbxPackages")
if stale then
	stale:Destroy()
end
local packagesRoot = roblox.import(`{root}/build/packages.rbxm`)[1]
packagesRoot.Parent = ServerStorage

local function load(name: string): any
	return require(packagesRoot:FindFirstChild(name) :: ModuleScript) :: any
end

return {
	jecs = load("jecs"),
	miu = load("miu"),
	planck = load("planck"),
	planckRunService = load("planck_run_service"),
	virtualList = load("miu_virtual_list"),
	wind = load("miu_wind"),
}
