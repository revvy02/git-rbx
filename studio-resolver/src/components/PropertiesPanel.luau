--[[
	Plain properties pane for the selected instance: prop names in #9cdcfe,
	typed value renderers, with attributes and tags in their own headed
	sections below the properties. No diff presentation yet — the conflict
	layers come back iteratively. One piece of conflict knowledge stays,
	silently: on the BASE tab a contested property shows the boot snapshot,
	since the live instance may be previewing a staged side.

	Studio offers no runtime property enumeration, so the listing is a curated
	per-class set (pcall-guarded) rather than exhaustive — attributes and tags
	ARE enumerable and complete.
]]

local Element = require("../packages").miu.Instance

local format = require("../format")
local theme = require("../theme")

local conflicts = require("../ecs/systems/conflicts")
local cts = require("../ecs/cts")
local world = require("../ecs/world")

local PropertyRow = require("./properties/PropertyRow")

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

-- Sub-header band: same treatment as the panel title, one notch shorter.
local function sectionHeader(id: string, text: string)
	Element("TextLabel", {
		id = id,
		Text = text,
		Size = UDim2.new(1, 0, 0, 20),
		BackgroundColor3 = theme.surfaceHigh,
		TextColor3 = theme.textMuted,
		TextSize = theme.headerSize,
		Font = theme.font,
	})
end

-- Tags are bare strings, not name/value pairs — a plain label row.
local function tagRow(id: string, tag: string, stripe: boolean)
	Element("TextLabel", {
		id = id,
		Text = tag,
		Size = UDim2.new(1, 0, 0, 14),
		BackgroundColor3 = theme.bg,
		BackgroundTransparency = if stripe then 0.5 else 1,
		TextColor3 = theme.text,
		TextXAlignment = Enum.TextXAlignment.Left,
		TextTruncate = Enum.TextTruncate.AtEnd,
		TextSize = theme.textSizeSmall,
		Font = theme.font,
	})
end

local function PropertiesPanel()
	Element("ScrollingFrame", {
		id = "properties",
		Position = UDim2.new(0, 0, 0.5, 0),
		Size = UDim2.new(1, 0, 0.5, 0),
		tags = { "bg-none", "flex-col" },
		ScrollingDirection = Enum.ScrollingDirection.Y,
		AutomaticCanvasSize = Enum.AutomaticSize.Y,
	}, function()
		local inst = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?

		Element("TextLabel", {
			id = "title",
			Text = if inst then `PROPERTIES — {inst.Name}` else "PROPERTIES",
			Size = UDim2.new(1, 0, 0, 24),
			BackgroundColor3 = theme.surfaceHigh,
			TextColor3 = theme.textMuted,
			TextSize = theme.headerSize,
			Font = theme.font,
			TextTruncate = Enum.TextTruncate.AtEnd,
		})

		if world:has(cts.Finished, cts.Finished) or not inst or not inst.Parent then
			return
		end

		-- BASE tab fidelity: the live target may be previewing a staged side;
		-- its contested property shows the boot snapshot instead.
		local contestedProperty: string? = nil
		local contestedBaseValue: any = nil
		if world:get(cts.ViewTab, cts.ViewTab) == "base" then
			for _, entity in conflicts.all() do
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

		Element("Frame", {
			id = "props",
			tags = { "bg-none", "auto-size-y", "flex-col", "gap-sm", "p-sm" },
		}, function()
			local rowIndex = 0
			local function row(id: string, name: string, value: any, propertyName: string?)
				rowIndex += 1
				PropertyRow(id, name, value, nil, propertyName, rowIndex % 2 == 0)
			end

			local nameValue = if contestedProperty == "Name" then contestedBaseValue else inst.Name
			row("ClassName", "ClassName", inst.ClassName)
			row("Name", "Name", nameValue)

			local seen: { [string]: boolean } = { Name = true, ClassName = true }
			for _, group in CLASS_PROPS do
				if inst:IsA(group.class) then
					for _, property in group.props do
						if not seen[property] then
							seen[property] = true
							local ok, value = pcall(function()
								return (inst :: any)[property]
							end)
							if ok then
								if contestedProperty == property then
									value = contestedBaseValue
								end
								row(property, property, value, property)
							end
						end
					end
				end
			end
		end)

		-- Attribute rows sort by name so the per-frame render is order-stable.
		local attributes = inst:GetAttributes()
		local attributeNames: { string } = {}
		for attribute in attributes do
			table.insert(attributeNames, attribute)
		end
		if #attributeNames > 0 then
			table.sort(attributeNames)
			sectionHeader("attributesHeader", "ATTRIBUTES")
			Element("Frame", {
				id = "attributes",
				tags = { "bg-none", "auto-size-y", "flex-col", "gap-sm", "p-sm" },
			}, function()
				for index, attribute in attributeNames do
					PropertyRow(
						`attr_{attribute}`,
						format.escapeRich(attribute),
						attributes[attribute],
						nil,
						nil,
						index % 2 == 0
					)
				end
			end)
		end

		local tags = inst:GetTags()
		if #tags > 0 then
			table.sort(tags)
			sectionHeader("tagsHeader", "TAGS")
			Element("Frame", {
				id = "tagList",
				tags = { "bg-none", "auto-size-y", "flex-col", "gap-sm", "p-sm" },
			}, function()
				for index, tag in tags do
					tagRow(`tag_{tag}`, tag, index % 2 == 0)
				end
			end)
		end
	end)
end

return PropertiesPanel
