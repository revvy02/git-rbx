local Element = require("@shared/packages").miu.Element

local theme = require("@shared/library/theme")
local Icon = require("@shared/ui/Icon")

local function Button(
	id: string,
	text: string,
	highlighted: boolean,
	color: Color3,
	onClick: () -> (),
	onView: (() -> ())?
)
	if not onView then
		Element("TextButton", {
			id = id,
			Text = text,
			Size = UDim2.fromOffset(64, 22),
			tags = { "corner-sm", "button" },
			BackgroundColor3 = if highlighted then color else theme.neutralButton,
			TextColor3 = if highlighted then theme.bg else theme.text,
			AutoButtonColor = true,
			events = { Activated = onClick },
		})
		return
	end

	-- Compound choice: the wide section stages/toggles the resolution, while
	-- the explicit View section stages this side without toggling and frames it.
	-- They are siblings inside one visual control rather than nested buttons,
	-- so activating View cannot bubble into the choice handler.
	Element("Frame", {
		id = id,
		Size = UDim2.fromOffset(90, 22),
		tags = { "bg-none", "flex-row", "gap-xs" },
	}, function()
		Element("TextButton", {
			id = "choose",
			Text = text,
			Size = UDim2.fromOffset(64, 22),
			tags = { "corner-sm", "button" },
			BackgroundColor3 = if highlighted then color else theme.neutralButton,
			TextColor3 = if highlighted then theme.bg else theme.text,
			AutoButtonColor = true,
			events = { Activated = onClick },
		})
		Element("TextButton", {
			id = "view",
			Text = "View",
			TextTransparency = 1,
			Size = UDim2.fromOffset(22, 22),
			tags = { "corner-sm", "button" },
			BackgroundColor3 = theme.neutralButton,
			AutoButtonColor = true,
			events = { Activated = onView },
		}, function()
			Icon("icon", "lucide:eye", color, UDim2.fromOffset(14, 14))
		end)
	end)
end

return Button
