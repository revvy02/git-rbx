--[[
	Vendored miu core (revvy02/miu 0.1.0 + retained-scope 0.1.0) with string
	requires so rodeo's darklua bundling can resolve it — the pesde shims use
	instance-path requires that don't survive single-file bundling.
]]
local Context = require("@self/Context")
local Instance = require("@self/Instance")
local Portal = require("@self/Portal")
local scope = require("@self/scope")

export type MiuNode<T = {}> = scope.MiuNode<T>

local Miu = setmetatable({
	push = scope.push,
	pop = scope.pop,
	peek = scope.peek,
	root = scope.root,
	Context = Context,
	Instance = Instance,
	Portal = Portal,
}, {
	__call = function(_, id, fn)
		return scope(id, fn)
	end,
})

return Miu
