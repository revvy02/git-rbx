--[[
	Properties pane for the selected instance, viewer-style: prop names in
	#9cdcfe, values in mono, attributes and tags included. When the selected
	instance is a conflict target the comparison card docks at the top, and a
	conflicted property's name carries the conflict's status color.

	Studio offers no runtime property enumeration, so the listing is a curated
	per-class set (pcall-guarded) rather than exhaustive — attributes and tags
	ARE enumerable and complete.
]]

local Element = require("../miu/Instance")

local format = require("../format")
local state = require("../state")
local theme = require("../theme")

local ComparisonCard = require("./ComparisonCard")

type Conflict = state.Conflict

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

local function propValueText(v: any): string
	local t = typeof(v)
	if v == nil then
		return "nil"
	elseif t == "Instance" then
		return format.escapeRich((v :: Instance).Name)
	elseif t == "EnumItem" then
		return (v :: EnumItem).Name
	end
	return format.formatValue(v)
end

local function PropRow(id: string, name: string, valueText: string, nameHex: string?)
	Element("TextLabel", {
		id = id,
		RichText = true,
		Text = `<font color="{nameHex or theme.propNameHex}">{name}</font>  {valueText}`,
		tags = { "text-base", "auto-size-y" },
		TextColor3 = theme.text,
		TextXAlignment = Enum.TextXAlignment.Left,
		TextSize = theme.textSizeSmall,
		Font = theme.font,
		TextWrapped = true,
	})
end

local function PropertiesPanel()
	Element("ScrollingFrame", {
		id = "properties",
		Position = UDim2.new(0, 16, 0.5, 0),
		Size = UDim2.new(0, 300, 0.5, -16),
		tags = { "bg-none", "flex-col", "gap-sm", "pr-md" },
		ScrollingDirection = Enum.ScrollingDirection.Y,
		AutomaticCanvasSize = Enum.AutomaticSize.Y,
	}, function()
		local inst = state.selectedInstance

		Element("TextLabel", {
			id = "title",
			Text = if inst then `PROPERTIES — {inst.Name}` else "PROPERTIES",
			Size = UDim2.new(1, 0, 0, 24),
			tags = { "corner-sm", "surface", "vs-border" },
			TextColor3 = theme.textMuted,
			TextSize = theme.headerSize,
			Font = theme.font,
			TextTruncate = Enum.TextTruncate.AtEnd,
		})

		if state.finished or not inst or not inst.Parent then
			return
		end

		local conflict: Conflict? = nil
		for _, candidate in state.conflicts do
			if candidate.target == inst then
				conflict = candidate
				break
			end
		end
		if conflict then
			ComparisonCard(conflict)
		end

		Element("Frame", {
			id = "props",
			tags = { "auto-size-y", "flex-col", "gap-xs", "p-md", "corner-sm", "surface", "vs-border" },
		}, function()
			local conflictedProperty = if conflict and conflict.kind == "Property" then conflict.property else nil
			local conflictHex = if conflict then theme.toHex(state.statusColor(conflict)) else nil

			PropRow("ClassName", "ClassName", inst.ClassName)
			PropRow(
				"Name",
				"Name",
				format.escapeRich(inst.Name),
				if conflictedProperty == "Name" then conflictHex else nil
			)

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
								PropRow(
									property,
									property,
									propValueText(value),
									if conflictedProperty == property then conflictHex else nil
								)
							end
						end
					end
				end
			end

			for attribute, value in inst:GetAttributes() do
				PropRow(`attr_{attribute}`, `@{format.escapeRich(attribute)}`, propValueText(value))
			end

			local tags = inst:GetTags()
			if #tags > 0 then
				PropRow("tags", "Tags", format.escapeRich(table.concat(tags, ", ")))
			end
		end)
	end)
end

return PropertiesPanel
