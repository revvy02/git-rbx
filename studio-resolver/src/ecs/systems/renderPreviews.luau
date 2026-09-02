--[[
	Staged-selection preview. Boot stashes the stamped container out of the
	scene and fills each conflict entity's side info (Ours/Theirs folders) and
	Base snapshot (what unpicking restores); the scheduled loop renders the
	staged selection into the scene — staging Ours renders ours' version,
	Theirs renders theirs', unpicked renders base. Property values apply to
	the target, moves reparent it, and decisions that require serialized-only
	state swap stamped clones in and out. None of this touches the disk file —
	the CLI at Complete is the
	only writer. Staged Resolution vs Rendered drives the frame diff.
]]

local packages = require("../../packages")

local format = require("../../library/format")
local types = require("../types")

type SideInfo = types.SideInfo
type Base = types.Base

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
	local conflictList = (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}
	local automaticPivotPlan = (world:get(cts.PivotPlan, cts.PivotPlan) :: { types.PivotApplication }?) or {}

	local function eachPart(root: Instance, fn: (BasePart) -> ())
		if root:IsA("BasePart") then
			fn(root)
		end
		for _, descendant in root:GetDescendants() do
			if descendant:IsA("BasePart") then
				fn(descendant)
			end
		end
	end

	local function eachModel(root: Instance, fn: (Model) -> ())
		if root:IsA("Model") then
			fn(root)
		end
		for _, descendant in root:GetDescendants() do
			if descendant:IsA("Model") then
				fn(descendant)
			end
		end
	end

	local function capturePartTransparencies(root: Instance): { [BasePart]: number }
		local parts: { [BasePart]: number } = {}
		eachPart(root, function(part)
			parts[part] = part.Transparency
		end)
		return parts
	end

	local function sideInfo(entry: Instance, sideName: string): SideInfo
		local info: SideInfo = {}
		local folder = entry:FindFirstChild(sideName)
		if folder then
			info.folder = folder
			for _, child in folder:GetChildren() do
				if child.Name == "Destination" and child:IsA("ObjectValue") then
					info.destination = child.Value
				elseif child.Name == "MoveOuts" then
					local moveOuts = {}
					for _, record in child:GetChildren() do
						if record.Name == "References" then
							continue
						end
						local destinationValue = record:FindFirstChild("Destination")
						local destination = if destinationValue and destinationValue:IsA("ObjectValue")
							then destinationValue.Value
							else nil
						local snapshot = nil
						for _, candidate in record:GetChildren() do
							if not candidate:IsA("ObjectValue") then
								snapshot = candidate
								break
							end
						end
						if destination and snapshot then
							table.insert(moveOuts, {
								folder = record,
								snapshot = snapshot,
								destination = destination,
							})
						end
					end
					info.moveOuts = moveOuts
				elseif child.Name == "PivotPlan" then
					local pivots: { types.PivotApplication } = {}
					for _, pivot in child:GetChildren() do
						local targetValue = pivot:FindFirstChild("Target")
						local target = if targetValue and targetValue:IsA("ObjectValue") then targetValue.Value else nil
						local order = pivot:GetAttribute("PivotOrder") :: number?
						local parentOrder = pivot:GetAttribute("PivotParentOrder") :: number?
						local delta = pivot:GetAttribute("Delta") :: CFrame?
						if target and order and delta then
							table.insert(pivots, {
								target = target,
								order = order,
								parentOrder = parentOrder,
								delta = delta,
							})
						end
					end
					info.pivots = pivots
				elseif child.Name ~= "__RbxDiffImpact" then
					info.clone = child
				end
			end
			info.deleted = folder:GetAttribute("Deleted") :: boolean?
			info.destinationPath = folder:GetAttribute("DestinationPath") :: string?
			info.delta = folder:GetAttribute("Delta") :: CFrame?
		end
		return info
	end

	-- Boot pass: the stamped container rides inside model files, so a model
	-- preview imports it into workspace — where every clone renders on top
	-- of its target. It is pure data for this session: stash it out of the
	-- scene (copy-only; the file keeps its container until finalize strips
	-- it), then per conflict entity set side clones + the base snapshot.
	local container = world:get(cts.Container, cts.Container) :: Instance
	if container:IsDescendantOf(workspace) then
		container.Parent = game:GetService("ServerStorage")
	end

	-- Capture one canonical snapshot across every automatic and selectable
	-- frame boundary. Overlapping boundaries intentionally share entries in
	-- these maps: restoration writes the same canonical value exactly once.
	local canonicalFrameParts: { [BasePart]: CFrame } = {}
	local canonicalFrameModels: { [Model]: CFrame } = {}
	local function captureFrameTarget(target: Instance)
		eachPart(target, function(part)
			canonicalFrameParts[part] = canonicalFrameParts[part] or part.CFrame
		end)
		eachModel(target, function(model)
			canonicalFrameModels[model] = canonicalFrameModels[model] or model.WorldPivot
		end)
	end
	for _, application in automaticPivotPlan do
		captureFrameTarget(application.target)
	end

	for _, entity in conflictList do
		local entry = world:get(entity, cts.Entry) :: Instance
		local ours = sideInfo(entry, "Ours")
		local theirs = sideInfo(entry, "Theirs")
		world:set(entity, cts.Ours, ours)
		world:set(entity, cts.Theirs, theirs)
		world:set(entity, cts.Rendered, "__init")
		for _, info in { ours, theirs } do
			for _, application in info.pivots or {} do
				captureFrameTarget(application.target)
			end
		end

		local target = world:get(entity, cts.Target) :: Instance
		local kind = world:get(entity, cts.Kind)
		local base: Base = {}
		if kind == "Property" then
			local property = world:get(entity, cts.Property) :: string
			if property == "Name" then
				base.value = target.Name
			elseif property == "WorldPivotData" and target:IsA("Model") then
				base.value = target.WorldPivot
			else
				local ok, value = format.readProperty(target, property)
				base.value = if ok then value else nil
			end
		elseif kind == "PropertyBundle" or kind == "DeleteVsEdit" then
			-- Both kinds use clone swaps. Mesh geometry includes serialized-only
			-- InitialSize, so property writes cannot faithfully preview its bundle.
			base.parts = capturePartTransparencies(target)
		elseif kind == "MoveTarget" then
			base.parent = target.Parent
		elseif kind == "Pivot" then
			captureFrameTarget(target)
		end
		world:set(entity, cts.Base, base)
	end

	-- Render the staged selection; nil (unpicked) restores the base snapshot.
	local function applyPreview(entity: any)
		local staged = world:get(entity, cts.Resolution) :: string?
		local kind = world:get(entity, cts.Kind)
		local target = world:get(entity, cts.Target) :: Instance
		local ours = world:get(entity, cts.Ours) :: SideInfo
		local theirs = world:get(entity, cts.Theirs) :: SideInfo
		local base = world:get(entity, cts.Base) :: Base

		if kind == "Property" then
			if staged ~= "custom" then
				local sideClone = if staged == "ours"
					then ours.clone
					elseif staged == "theirs" then theirs.clone
					else nil
				local property = world:get(entity, cts.Property) :: string
				if property == "WorldPivotData" and target:IsA("Model") then
					-- WorldPivotData itself is serialized-only. WorldPivot exposes
					-- the same pivot and, unlike PivotTo, assigning it deliberately
					-- does not move descendants.
					if sideClone and sideClone:IsA("Model") then
						target.WorldPivot = sideClone.WorldPivot
					elseif staged == nil and base.value ~= nil then
						target.WorldPivot = base.value
					end
				elseif sideClone then
					pcall(function()
						(target :: any)[property] = (sideClone :: any)[property]
					end)
				elseif staged == nil and base.value ~= nil then
					pcall(function()
						(target :: any)[property] = base.value
					end)
				end
			end
		elseif kind == "PropertyBundle" then
			local selected: SideInfo? = if staged == "ours" then ours elseif staged == "theirs" then theirs else nil
			local function placeClone(side: string, info: SideInfo)
				local clone = info.clone
				if clone then
					-- Preserve independently staged placement/scale while borrowing the
					-- clone's inseparable mesh asset + InitialSize payload.
					if clone:IsA("BasePart") and target:IsA("BasePart") then
						clone.CFrame = target.CFrame
						clone.Size = target.Size
					end
					clone.Parent = if staged == side then target.Parent else info.folder
				end
			end
			placeClone("ours", ours)
			placeClone("theirs", theirs)
			local hideTarget = selected ~= nil and selected.clone ~= nil
			for part, original in base.parts :: { [BasePart]: number } do
				part.Transparency = if hideTarget then 1 else original
			end
		elseif kind == "MoveTarget" then
			local destination = if staged == "ours"
				then ours.destination
				elseif staged == "theirs" then theirs.destination
				else nil
			-- A side with no live destination (moved into the other branch's added
			-- content) previews as base rather than sticking at a stale parent.
			destination = destination or base.parent
			if destination then
				target.Parent = destination
			end
		elseif kind == "DeleteVsEdit" then
			-- One side deletes (stamped Deleted attribute); the other keeps —
			-- an edited clone, or a move out of the deleted subtree (no clone).
			-- Staging the delete side hides the base target; staging a keep
			-- side with an edited clone swaps the clone into the target's spot
			-- (it carries its own true coordinates). A clone-less keep side
			-- just keeps the target visible.
			local deleteSide = if ours.deleted then "ours" elseif theirs.deleted then "theirs" else nil
			local keepSide = if deleteSide == "ours" then "theirs" else "ours"
			local keepInfo = if keepSide == "ours" then ours else theirs
			local clone = keepInfo.clone
			local moveOuts = keepInfo.moveOuts or {}
			local hideTarget = staged == deleteSide or ((clone ~= nil or #moveOuts > 0) and staged == keepSide)
			for part, original in base.parts :: { [BasePart]: number } do
				part.Transparency = if hideTarget then 1 else original
			end
			if clone then
				clone.Parent = if staged == keepSide then target.Parent else keepInfo.folder
			end
			for _, moveOut in moveOuts do
				moveOut.snapshot.Parent = if staged == keepSide then moveOut.destination else moveOut.folder
			end
		end
	end

	local function restoreCanonicalFrames()
		for part, cframe in canonicalFrameParts do
			if part.Parent then
				part.CFrame = cframe
			end
		end
		for model, pivot in canonicalFrameModels do
			if model.Parent then
				model.WorldPivot = pivot
			end
		end
	end

	local function applyPivotPlan()
		local applications: { types.PivotApplication } = table.clone(automaticPivotPlan)
		for _, entity in conflictList do
			local staged = world:get(entity, cts.Resolution) :: string?
			local info = if staged == "ours"
				then world:get(entity, cts.Ours) :: SideInfo
				elseif staged == "theirs" then world:get(entity, cts.Theirs) :: SideInfo
				else nil
			if info then
				if world:get(entity, cts.Kind) == "Pivot" and info.delta then
					table.insert(applications, {
						target = world:get(entity, cts.Target) :: Instance,
						order = (world:get(entity, cts.PivotOrder) :: number?) or math.huge,
						parentOrder = world:get(entity, cts.PivotParentOrder) :: number?,
						delta = info.delta,
					})
				end
				for _, pivot in info.pivots or {} do
					table.insert(applications, pivot)
				end
			end
		end
		table.sort(applications, function(a, b)
			return a.order < b.order
		end)

		-- Compose every local choice first. Direct property writes then visit
		-- each descendant once, avoiding both compounded preview transforms and
		-- float drift from repeated parent/child PivotTo calls.
		local effectiveByOrder: { [number]: CFrame } = {}
		local effectiveByTarget: { [Instance]: CFrame } = {}
		local roots: { Instance } = {}
		for _, application in applications do
			local parent = if application.parentOrder then effectiveByOrder[application.parentOrder] else nil
			local effective = application.delta * (parent or CFrame.identity)
			effectiveByOrder[application.order] = effective
			effectiveByTarget[application.target] = effective
			if parent == nil then
				table.insert(roots, application.target)
			end
		end

		local function visit(instance: Instance, inherited: CFrame?)
			local effective = effectiveByTarget[instance] or inherited
			if effective then
				if instance:IsA("BasePart") then
					instance.CFrame = effective * instance.CFrame
				elseif instance:IsA("Model") then
					-- Assigning WorldPivot directly changes only the serialized pivot;
					-- descendants receive their own one-time world transforms below.
					instance.WorldPivot = effective * instance.WorldPivot
				end
			end
			for _, child in instance:GetChildren() do
				visit(child, effective)
			end
		end
		for _, root in roots do
			if root.Parent then
				visit(root, nil)
			end
		end
	end

	scheduler:addSystem({
		name = "render_previews",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			if world:has(cts.Finished, cts.Finished) then
				return
			end
			debug.profilebegin("rbxdiff_previews")
			local changed = false
			for _, entity in conflictList do
				local staged = (world:get(entity, cts.Resolution) :: string?) or "unresolved"
				if world:get(entity, cts.Rendered) ~= staged then
					changed = true
					break
				end
			end
			if changed then
				-- Rebuild from the canonical frame on every decision change so
				-- switching ours/theirs never compounds transforms. Ordinary
				-- property/tree choices run first; the complete hierarchical frame
				-- plan then runs once in parent-before-child order.
				restoreCanonicalFrames()
				for _, entity in conflictList do
					if world:get(entity, cts.Kind) ~= "Pivot" then
						applyPreview(entity)
					end
				end
				applyPivotPlan()
				for _, entity in conflictList do
					local staged = (world:get(entity, cts.Resolution) :: string?) or "unresolved"
					world:set(entity, cts.Rendered, staged)
				end
			end
			debug.profileend()
		end,
	})
end
