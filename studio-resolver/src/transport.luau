--[[
	Decision transport: the canonical CLI, host-side. Complete batches every
	staged decision through `rbx-diff resolve` and finalizes; the disk file is
	only ever written here.
]]

local HttpService = game:GetService("HttpService")

local process = require("@rodeo/process")

local state = require("@self/state")

local transport = {}

-- Commit every staged decision through the canonical CLI, then finalize.
function transport.complete()
	if state.completing or state.finished then
		return
	end
	state.completing = true
	state.completeError = nil
	task.spawn(function()
		for _, conflict in state.conflicts do
			local args
			if conflict.resolved == "custom" then
				-- Capture the live value NOW — the final state of the user's
				-- in-Studio edit is the resolution
				local encoded = state.encodeValue(state.liveValue(conflict))
				args = {
					state.rbxDiffBin,
					"resolve",
					state.mergedFile,
					"--take",
					"custom",
					"--entry",
					conflict.name,
					"--value",
					HttpService:JSONEncode(encoded),
				}
			else
				args = {
					state.rbxDiffBin,
					"resolve",
					state.mergedFile,
					"--take",
					conflict.resolved :: string,
					"--entry",
					conflict.name,
				}
			end
			local ok, result = pcall(process.run, args)
			if not ok or not result.ok then
				state.completeError = `--take {conflict.name} failed: {if ok then result.err else result}`
				warn(state.completeError)
				state.completing = false
				return
			end
		end
		local ok, result = pcall(process.run, { state.rbxDiffBin, "resolve", state.mergedFile, "--finalize" })
		if ok and result.ok then
			state.finished = true
		else
			state.completeError = `finalize failed: {if ok then result.err else result}`
			warn(state.completeError)
		end
		state.completing = false
	end)
end

return transport
