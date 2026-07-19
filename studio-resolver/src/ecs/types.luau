--[[
	Arg types for system init functions, after game-prototype's ecs/types.luau.
	The instance-require package loader erases jecs/planck types, so World and
	Scheduler are opaque; Cts keeps its keyed shape.
]]

local cts = require("./cts")

export type World = any -- jecs.World
export type Cts = typeof(cts)
export type Scheduler = any -- Planck.Scheduler

export type SideInfo = cts.SideInfo
export type Base = cts.Base

export type SystemInit = (world: World, cts: Cts, scheduler: Scheduler) -> ()

return nil
