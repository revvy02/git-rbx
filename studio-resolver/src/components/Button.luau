local Element = require("../packages").miu.Instance

local theme = require("../theme")

local function Button(id: string, text: string, highlighted: boolean, color: Color3, onClick: () -> ())
	Element("TextButton", {
		id = id,
		Text = text,
		Size = UDim2.new(0, 64, 0, 22),
		tags = { "corner-sm" },
		BackgroundColor3 = if highlighted then color else theme.neutralButton,
		TextColor3 = if highlighted then theme.bg else theme.text,
		TextSize = theme.textSizeSmall,
		Font = theme.font,
		AutoButtonColor = true,
		events = { Activated = onClick },
	})
end

return Button
