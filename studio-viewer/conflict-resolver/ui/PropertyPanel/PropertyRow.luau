local packages = require("@shared/packages")
local Element = packages.miu.Element
local tags = packages.wind.tags

local PropertyValue = require("./PropertyValue")

-- One property: name column (prop-blue) + the typed value renderer.
-- stripe = zebra shading.
local function PropertyRow(id: string, name: string, value: any, propertyName: string?, stripe: boolean?)
	Element("Frame", {
		id = id,
		tags = tags("row", "auto-size-y", "flex-row", "gap-sm", stripe and "stripe"),
	}, function()
		Element("TextLabel", {
			id = "name",
			Text = name,
			Size = UDim2.new(0, 104, 0, 14),
			tags = { "text", "text-small", "prop-name" },
			TextTruncate = Enum.TextTruncate.AtEnd,
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
