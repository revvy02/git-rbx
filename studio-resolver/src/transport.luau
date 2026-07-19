--[[
	Decision transport: the canonical CLI, host-side. Complete batches every
	staged decision through `rbx-diff resolve` and finalizes; the disk file is
	only ever written here. Progress/outcome land in the world as the
	Completing/Finished tags and the CompleteError singleton.
]]

local HttpService = game:GetService("HttpService")

local process = require("@rodeo/process")

local config = require("@self/config")
local format = require("@self/format")

local conflicts = require("@self/ecs/library/conflicts")
local cts = require("@self/ecs/cts")
local world = require("@self/ecs/world")

local transport = {}

-- Commit every staged decision through the canonical CLI, then finalize.
function transport.complete()
	if world:has(cts.Completing, cts.Completing) or world:has(cts.Finished, cts.Finished) then
		return
	end
	world:add(cts.Completing, cts.Completing)
	world:remove(cts.CompleteError, cts.CompleteError)
	task.spawn(function()
		for _, entity in conflicts.all() do
			local entryName = (world:get(entity, cts.Entry) :: Instance).Name
			local resolution = world:get(entity, cts.Resolution) :: string?
			local args
			if resolution == "custom" then
				-- Capture the live value NOW — the final state of the user's
				-- in-Studio edit is the resolution
				local encoded = format.encodeValue(conflicts.liveValue(entity))
				args = {
					config.rbxDiffBin,
					"resolve",
					config.mergedFile,
					"--take",
					"custom",
					"--entry",
					entryName,
					"--value",
					HttpService:JSONEncode(encoded),
				}
			else
				args = {
					config.rbxDiffBin,
					"resolve",
					config.mergedFile,
					"--take",
					resolution :: string,
					"--entry",
					entryName,
				}
			end
			local ok, result = pcall(process.run, args)
			if not ok or not result.ok then
				local message = `--take {entryName} failed: {if ok then result.err else result}`
				world:set(cts.CompleteError, cts.CompleteError, message)
				warn(message)
				world:remove(cts.Completing, cts.Completing)
				return
			end
		end
		local ok, result = pcall(process.run, { config.rbxDiffBin, "resolve", config.mergedFile, "--finalize" })
		if ok and result.ok then
			world:add(cts.Finished, cts.Finished)
		else
			local message = `finalize failed: {if ok then result.err else result}`
			world:set(cts.CompleteError, cts.CompleteError, message)
			warn(message)
		end
		world:remove(cts.Completing, cts.Completing)
	end)
end

return transport
