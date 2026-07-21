--[[
	Launch configuration — set once by init.luau from CLI args, read by
	transport. Config, not state: nothing here changes during the session,
	so it stays a plain module instead of living in the world.
]]

local config = {
	mergedFile = "",
	rbxDiffBin = "rbx-diff",
}

return config
