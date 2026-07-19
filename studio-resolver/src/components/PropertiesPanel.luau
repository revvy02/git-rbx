--[[
	Plain properties pane for the selected instance: prop names in #9cdcfe,
	typed value renderers, attributes and tags included. No diff presentation
	yet — the conflict layers come back iteratively. One piece of conflict
	knowledge stays, silently: on the BASE tab a contested property shows the
	boot snapshot, since the live instance may be previewing a staged side.

	Studio offers no runtime property enumeration, so the listing is a curated
	per-class set (pcall-guarded) rather than exhaustive — attributes and tags
	ARE enumerable and complete.
]]

local Element = require("../miu/Instance")

local format = require("../format")
local state = require("../state")
local theme = require("../theme")

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

		-- BASE tab fidelity: the live target may be previewing a staged side;
		-- its contested property shows the boot snapshot instead.
		local conflict: Conflict? = nil
		if state.viewTab == "base" then
			for _, candidate in state.conflicts do
				if candidate.target == inst then
					conflict = candidate
					break
				end
			end
		end

		Element("Frame", {
			id = "content",
			tags = { "bg-none", "auto-size-y", "flex-col", "gap-sm", "p-sm" },
		}, function()
			local rowIndex = 0
			local function row(id: string, name: string, value: any, nameHex: string?, propertyName: string?)
				rowIndex += 1
				PropertyRow(id, name, value, nameHex, propertyName, rowIndex % 2 == 0)
			end

			local contestedProperty = if conflict and conflict.kind == "Property" then conflict.property else nil

			local nameValue = if contestedProperty == "Name" then (conflict :: Conflict).baseValue else inst.Name
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
									value = (conflict :: Conflict).baseValue
								end
								row(property, property, value, nil, property)
							end
						end
					end
				end
			end

			for attribute, value in inst:GetAttributes() do
				row(`attr_{attribute}`, `@{format.escapeRich(attribute)}`, value)
			end

			local tags = inst:GetTags()
			if #tags > 0 then
				row("tags", "Tags", table.concat(tags, ", "))
			end
		end)
	end)
end

return PropertiesPanel
