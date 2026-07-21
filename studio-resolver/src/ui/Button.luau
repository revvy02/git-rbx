local Element = require("../packages").miu.Element

local theme = require("../library/theme")

local function Button(id: string, text: string, highlighted: boolean, color: Color3, onClick: () -> ())
	Element("TextButton", {
		id = id,
		Text = text,
		Size = UDim2.new(0, 64, 0, 22),
		tags = { "corner-sm", "button" },
		BackgroundColor3 = if highlighted then color else theme.neutralButton,
		TextColor3 = if highlighted then theme.bg else theme.text,
		AutoButtonColor = true,
		events = { Activated = onClick },
	})
end

return Button
