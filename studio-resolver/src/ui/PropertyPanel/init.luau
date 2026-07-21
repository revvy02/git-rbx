--[[
	Plain properties pane for the selected instance: prop names in #9cdcfe,
	typed value renderers, with attributes and tags in their own headed
	sections below the properties. No diff presentation yet — the conflict
	layers come back iteratively. One piece of conflict knowledge stays,
	silently: on the BASE tab a contested property shows the boot snapshot,
	since the live instance may be previewing a staged side.

	Rows render through VirtualList: the pane flattens to a positioned item
	array (section headers + prop rows + tag rows, each with a precomputed
	y/height) and only the visible slice builds UI.

	Studio offers no runtime property enumeration, so the listing is a curated
	per-class set (pcall-guarded) rather than exhaustive — attributes and tags
	ARE enumerable and complete.
]]

local packages = require("../packages")
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions
local tags = packages.wind.tags

local format = require("../library/format")

local conflicts = require("../ecs/systems/library/conflicts")
local cts = require("../ecs/cts")
local world = require("../ecs/world")

local PropertyRow = require("@self/PropertyRow")

local CLASS_PROPS: { { class: string, props: { string } } } = {
	{
		class = "BasePart",
		props = {
			"Position",
			"Orientation",
			"Size",
			"Color",
			"Material",
			"Transparency",
			"Reflectance",
			"Anchored",
			"CanCollide",
		},
	},
	{ class = "Model", props = { "PrimaryPart" } },
	{ class = "Light", props = { "Brightness", "Color", "Enabled" } },
	{ class = "Camera", props = { "CFrame", "Focus", "FieldOfView" } },
	{ class = "Decal", props = { "Texture", "Color3", "Transparency" } },
	{ class = "Sound", props = { "SoundId", "Volume", "PlaybackSpeed" } },
	{ class = "ValueBase", props = { "Value" } },
}

local HEADER_HEIGHT = 24
local SECTION_HEIGHT = 20
local ROW_HEIGHT = 18 -- 14px row + the old gap-sm breathing room
local ASSET_ROW_HEIGHT = 36 -- 32px thumbnail row

type Item = {
	y: number,
	h: number,
	kind: "section" | "prop" | "tag",
	name: string,
	value: any,
	propertyName: string?,
	stripe: boolean,
}

-- Flatten the pane into positioned items (prefix-summed y offsets), zebra
-- restarting per section.
local function buildItems(inst: Instance): { Item }
	local items: { Item } = {}
	local offset = 4
	local rowIndex = 0

	local function push(kind: "section" | "prop" | "tag", h: number, name: string, value: any, propertyName: string?)
		local stripe = false
		if kind == "section" then
			rowIndex = 0
		else
			rowIndex += 1
			stripe = rowIndex % 2 == 0
		end
		table.insert(items, {
			y = offset,
			h = h,
			kind = kind,
			name = name,
			value = value,
			propertyName = propertyName,
			stripe = stripe,
		})
		offset += h
	end

	local function pushProp(name: string, value: any, propertyName: string?)
		local h = if format.isVisualAsset(propertyName, value) then ASSET_ROW_HEIGHT else ROW_HEIGHT
		push("prop", h, name, value, propertyName)
	end

	-- BASE tab fidelity: the live target may be previewing a staged side;
	-- its contested property shows the boot snapshot instead.
	local contestedProperty: string? = nil
	local contestedBaseValue: any = nil
	if world:get(cts.ViewTab, cts.ViewTab) == "base" then
		for _, entity in conflicts.list() do
			if world:get(entity, cts.Target) == inst then
				if world:get(entity, cts.Kind) == "Property" then
					contestedProperty = world:get(entity, cts.Property) :: string?
					local base = world:get(entity, cts.Base) :: cts.Base?
					contestedBaseValue = if base then base.value else nil
				end
				break
			end
		end
	end

	pushProp("ClassName", inst.ClassName)
	pushProp("Name", if contestedProperty == "Name" then contestedBaseValue else inst.Name)

	local seen: { [string]: boolean } = { Name = true, ClassName = true }
	for _, group in CLASS_PROPS do
		if inst:IsA(group.class) then
			for _, property in group.props do
				if not seen[property] then
					seen[property] = true
					local ok, value = format.readProperty(inst, property)
					if ok then
						if contestedProperty == property then
							value = contestedBaseValue
						end
						pushProp(property, value, property)
					end
				end
			end
		end
	end

	-- Attribute rows sort by name so the per-frame render is order-stable.
	local attributes = inst:GetAttributes()
	local attributeNames: { string } = {}
	for attribute in attributes do
		table.insert(attributeNames, attribute)
	end
	if #attributeNames > 0 then
		table.sort(attributeNames)
		push("section", SECTION_HEIGHT, "ATTRIBUTES", nil)
		for _, attribute in attributeNames do
			pushProp(attribute, attributes[attribute])
		end
	end

	local instanceTags = inst:GetTags()
	if #instanceTags > 0 then
		table.sort(instanceTags)
		push("section", SECTION_HEIGHT, "TAGS", nil)
		for _, tag in instanceTags do
			push("tag", ROW_HEIGHT, tag, nil)
		end
	end

	return items
end

local itemDimensions = Dimensions.getter(function(item: Item)
	return {
		size = UDim2.new(1, 0, 0, item.h),
		position = UDim2.fromOffset(0, item.y),
	}
end)

local function renderItem(item: Item)
	if item.kind == "section" then
		Element("TextLabel", {
			id = "header",
			Text = item.name,
			Size = UDim2.fromScale(1, 1),
			tags = { "header-band" },
		})
		return
	end

	Element("UIPadding", {
		id = "pad",
		PaddingLeft = UDim.new(0, 4),
		PaddingRight = UDim.new(0, 4),
	})
	-- Slot children need distinct ids per kind: VirtualList reuses slots
	-- across items, and miu reuses children by id — a Frame left by a prop
	-- row must not be handed TextLabel props when the slot becomes a tag row.
	if item.kind == "prop" then
		PropertyRow("prop", item.name, item.value, item.propertyName, item.stripe)
	else
		Element("TextLabel", {
			id = "tag",
			Text = item.name,
			Size = UDim2.new(1, 0, 0, 14),
			tags = tags("row", "text", "text-small", item.stripe and "stripe"),
			TextTruncate = Enum.TextTruncate.AtEnd,
		})
	end
end

local function PropertyPanel()
	Element("Frame", {
		id = "properties",
		Position = UDim2.new(0, 0, 0.5, 0),
		Size = UDim2.new(1, 0, 0.5, 0),
		tags = { "bg-none", "flex-col" },
	}, function()
		local inst = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?

		Element("TextLabel", {
			id = "title",
			Text = if inst then `PROPERTIES — {inst.Name}` else "PROPERTIES",
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
			TextTruncate = Enum.TextTruncate.AtEnd,
		})

		if world:has(cts.Finished, cts.Finished) or not inst or not inst.Parent then
			return
		end

		debug.profilebegin("rbxdiff_props_items")
		local items = buildItems(inst)
		debug.profileend()

		VirtualList({
			id = "rows",
			data = items,
			dimensions = itemDimensions,
			direction = "y",
			Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT),
			tags = { "bg-none" },
		}, function(item: Item)
			renderItem(item)
		end)
	end)
end

return PropertyPanel
