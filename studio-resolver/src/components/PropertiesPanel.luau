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
local PropertyRow = require("./properties/PropertyRow")

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

local function PropertiesPanel()
	Element("ScrollingFrame", {
		id = "properties",
		Position = UDim2.new(0, 0, 0.5, 0),
		Size = UDim2.new(1, 0, 0.5, 0),
		tags = { "bg-none", "flex-col" },
		ScrollingDirection = Enum.ScrollingDirection.Y,
		AutomaticCanvasSize = Enum.AutomaticSize.Y,
	}, function()
		local inst = state.selectedInstance

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

		if state.finished or not inst or not inst.Parent then
			return
		end

		-- The card follows the target AND its side clones, so browsing a
		-- version under the OURS/THEIRS tabs keeps the staging controls up.
		local conflict: Conflict? = nil
		for _, candidate in state.conflicts do
			local ours = candidate.ours
			local theirs = candidate.theirs
			if candidate.target == inst or (ours and ours.clone == inst) or (theirs and theirs.clone == inst) then
				conflict = candidate
				break
			end
		end

		Element("Frame", {
			id = "content",
			tags = { "bg-none", "auto-size-y", "flex-col", "gap-sm", "p-sm" },
		}, function()
			if conflict then
				ComparisonCard(conflict :: Conflict)
			end

			local conflictedProperty = if conflict and conflict.kind == "Property" then conflict.property else nil
			local conflictHex = if conflict then theme.toHex(state.statusColor(conflict)) else nil

			-- BASE tab + conflicted target: the live instance may be previewing
			-- a staged side, but base is what this tab claims — show the boot
			-- snapshot for the contested property.
			local baseOverride = state.viewTab == "base" and conflict ~= nil and (conflict :: Conflict).target == inst

			local nameValue = if baseOverride and conflictedProperty == "Name"
				then (conflict :: Conflict).baseValue
				else inst.Name
			PropertyRow("ClassName", "ClassName", inst.ClassName)
			PropertyRow("Name", "Name", nameValue, if conflictedProperty == "Name" then conflictHex else nil)

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
								if baseOverride and conflictedProperty == property then
									value = (conflict :: Conflict).baseValue
								end
								PropertyRow(
									property,
									property,
									value,
									if conflictedProperty == property then conflictHex else nil,
									property
								)
							end
						end
					end
				end
			end

			for attribute, value in inst:GetAttributes() do
				PropertyRow(`attr_{attribute}`, `@{format.escapeRich(attribute)}`, value)
			end

			local tags = inst:GetTags()
			if #tags > 0 then
				PropertyRow("tags", "Tags", table.concat(tags, ", "))
			end
		end)
	end)
end

return PropertiesPanel
