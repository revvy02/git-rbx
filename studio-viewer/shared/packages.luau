--[[
	Package loading without a checkout: the pesde-installed Roblox packages
	are rojo-built into an rbxm at bundle time (mise run bundle) and embedded
	here as base64. At boot the bytes go through SerializationService, which
	accepts a standard rbxm at plugin identity, and the pesde instance-path
	shims resolve natively from the imported tree. Only string requires go
	through the bundler; require(<ModuleScript>) happens at runtime.
]]

local SerializationService = game:GetService("SerializationService")
local ServerStorage = game:GetService("ServerStorage")

local blob = require("@shared/generated/packages_blob")

local ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
local DECODE: { [number]: number } = {}
for index = 1, #ALPHABET do
	DECODE[string.byte(ALPHABET, index)] = index - 1
end

local function base64Decode(text: string): buffer
	local padding = 0
	if text:sub(-2) == "==" then
		padding = 2
	elseif text:sub(-1) == "=" then
		padding = 1
	end
	local size = (#text // 4) * 3 - padding
	local out = buffer.create(size)
	local offset = 0
	for i = 1, #text, 4 do
		local a, b, c, d = string.byte(text, i, i + 3)
		local va, vb = DECODE[a], DECODE[b]
		local vc = if c == 61 then 0 else DECODE[c]
		local vd = if d == 61 then 0 else DECODE[d]
		local triple = bit32.bor(bit32.lshift(va, 18), bit32.lshift(vb, 12), bit32.lshift(vc, 6), vd)
		if offset < size then
			buffer.writeu8(out, offset, bit32.band(bit32.rshift(triple, 16), 0xff))
		end
		if offset + 1 < size then
			buffer.writeu8(out, offset + 1, bit32.band(bit32.rshift(triple, 8), 0xff))
		end
		if offset + 2 < size then
			buffer.writeu8(out, offset + 2, bit32.band(triple, 0xff))
		end
		offset += 3
	end
	return out
end

local stale = ServerStorage:FindFirstChild("GitRbxPackages")
if stale then
	stale:Destroy()
end
local packagesRoot = SerializationService:DeserializeInstancesAsync(base64Decode(blob))[1]
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
