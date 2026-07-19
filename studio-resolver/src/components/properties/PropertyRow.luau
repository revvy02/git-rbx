local Element = require("../../miu/Instance")

local theme = require("../../theme")

local PropertyValue = require("./PropertyValue")

-- One property: name column (prop-blue, conflicted names carry the conflict's
-- status hex) + the typed value renderer.
local function PropertyRow(id: string, name: string, value: any, nameHex: string?, propertyName: string?)
	Element("Frame", {
		id = id,
		Size = UDim2.new(1, 0, 0, 0),
		AutomaticSize = Enum.AutomaticSize.Y,
		tags = { "bg-none", "flex-row", "gap-sm" },
	}, function()
		Element("TextLabel", {
			id = "name",
			RichText = true,
			Text = `<font color="{nameHex or theme.propNameHex}">{name}</font>`,
			Size = UDim2.new(0, 104, 0, 14),
			BackgroundTransparency = 1,
			TextColor3 = theme.text,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextTruncate = Enum.TextTruncate.AtEnd,
			TextSize = theme.textSizeSmall,
			Font = theme.font,
		})
		Element("Frame", {
			id = "value",
			Size = UDim2.new(1, -112, 0, 0),
			AutomaticSize = Enum.AutomaticSize.Y,
			tags = { "bg-none", "flex-col" },
		}, function()
			PropertyValue("typed", value, propertyName)
		end)
	end)
end

return PropertyRow
