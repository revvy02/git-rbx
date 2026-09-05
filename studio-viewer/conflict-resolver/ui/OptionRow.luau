local Element = require("@shared/packages").miu.Element

local theme = require("@shared/library/theme")

-- One row of the comparison card: side name + formatted value, tinted when
-- staged, click to stage.
local function OptionRow(id: string, name: string, value: string?, color: Color3, staged: boolean, onClick: () -> ())
	Element("TextButton", {
		id = id,
		Text = "",
		Size = UDim2.new(1, 0, 0, 0),
		AutomaticSize = Enum.AutomaticSize.Y,
		tags = { "corner-sm" },
		BackgroundColor3 = if staged then theme.selection else theme.neutralButton,
		BackgroundTransparency = if staged then 0 else 0.5,
		AutoButtonColor = true,
		events = { Activated = onClick },
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 6),
			PaddingRight = UDim.new(0, 6),
			PaddingTop = UDim.new(0, 3),
			PaddingBottom = UDim.new(0, 3),
		})
		Element("UIListLayout", {
			id = "layout",
			FillDirection = Enum.FillDirection.Horizontal,
			Padding = UDim.new(0, 8),
			VerticalAlignment = Enum.VerticalAlignment.Center,
		})
		Element("TextLabel", {
			id = "name",
			Text = name,
			Size = UDim2.new(0, 48, 0, 16),
			tags = { "text", "text-small" },
			TextColor3 = color,
		})
		Element("TextLabel", {
			id = "value",
			RichText = true,
			Text = value or "?",
			tags = { "text", "text-small", "auto-size-y" },
			TextWrapped = true,
		})
	end)
end

return OptionRow
