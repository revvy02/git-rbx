--[[
	Direct conflict patch for the selected conflict's staged (or default)
	side: that side's impact ops against the base and side manifests, through
	the shared document adapter and PatchVisualizer. A pivot conflict
	contributes a Pivot row carrying the side's delta.
]]

local Element = require("@shared/packages").miu.Element

local PatchVisualizer = require("@shared/ui/PatchVisualizer")
local fromDocument = require("@shared/ui/PatchVisualizer/fromDocument")
local conflicts = require("../ecs/systems/library/conflicts")
local cts = require("../ecs/cts")
local format = require("@shared/library/format")
local world = require("../ecs/world")

type ImpactSide = cts.ImpactSide
type SideInfo = cts.SideInfo
type VersionTrees = cts.VersionTrees

local HEADER_HEIGHT = 24

local function resolutionOf(entity: any): string?
	return if world:get(entity, cts.Members)
		then conflicts.groupResolution(entity)
		else world:get(entity, cts.Resolution) :: string?
end

local function buildPatch(entity: any, side: string): (PatchVisualizer.Patch, ImpactSide)
	local impact = conflicts.impact(entity, side)
	local trees = (world:get(cts.VersionTrees, cts.VersionTrees) :: VersionTrees?) or {}

	local pivots = {}
	local info = world:get(entity, if side == "ours" then cts.Ours else cts.Theirs) :: SideInfo?
	if info and info.delta then
		local target = world:get(entity, cts.Target) :: Instance?
		local ids = world:get(cts.VirtualSubjectIds, cts.VirtualSubjectIds) :: { [Instance]: number }?
		local id = if target and ids then ids[target] else nil
		if id then
			table.insert(pivots, {
				id = id,
				current = "base placement",
				incoming = format.escapeRich(format.formatDelta(info.delta)),
			})
		end
	end

	local patch = fromDocument({
		ops = impact.ops,
		old = trees.base or {},
		new = trees[side] or trees.base or {},
		pivots = pivots,
		addProperties = true,
	})
	return patch, impact
end

local function ImpactPanel()
	local entity = world:get(cts.SelectedConflict, cts.SelectedConflict)
	local resolution = entity and resolutionOf(entity) or nil
	local side = if resolution == "theirs" then "theirs" else "ours"

	local patch: PatchVisualizer.Patch = { nodes = {} }
	local impact: ImpactSide? = nil
	if entity then
		patch, impact = buildPatch(entity, side)
	end

	Element("Frame", {
		id = "impact",
		Size = UDim2.fromScale(1, 1),
		tags = { "surface", "vs-border" },
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = if impact
				then `DIRECT DIFF — {impact.instanceCount} CHANGED · {impact.propertyCount} {if impact.propertyCount
						== 1
					then "PROPERTY"
					else "PROPERTIES"}`
				else "DIRECT DIFF",
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
		})

		PatchVisualizer({
			patch = patch,
			stateKey = if entity then `{tostring(entity)}:{side}` else nil,
			headers = { "Property", "Base", string.upper(side:sub(1, 1)) .. side:sub(2) },
			selectedId = world:get(cts.SelectedExplorerId, cts.SelectedExplorerId) :: number?,
			onSelect = function(node)
				conflicts.selectExplorerId(node.id :: number)
			end,
			emptyText = "No direct changes on this side",
			Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT),
			Position = UDim2.fromOffset(0, HEADER_HEIGHT),
		})
	end)
end

return ImpactPanel
