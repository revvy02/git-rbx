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
export type ImpactOperation = cts.ImpactOperation
export type ImpactSide = cts.ImpactSide
export type Base = cts.Base
export type ExplorerNode = cts.ExplorerNode
export type ExplorerProjection = cts.ExplorerProjection
export type VersionTreeNode = cts.VersionTreeNode
export type VersionTrees = cts.VersionTrees
export type PivotApplication = cts.PivotApplication

export type SystemInit = (world: World, cts: Cts, scheduler: Scheduler) -> ()

return nil
