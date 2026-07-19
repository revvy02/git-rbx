local Element = require("../miu/Instance")

local state = require("../state")
local transport = require("../transport")

local ConflictRow = require("./ConflictRow")

-- The conflict list + Complete button. The scroller is the panel: invisible,
-- stretched to the bottom of the screen; rows and the button carry their own
-- backgrounds, and AutomaticCanvasSize extends past the viewport when content
-- overflows.
local function Panel()
	local unresolved = 0
	for _, conflict in state.conflicts do
		if not conflict.resolved then
			unresolved += 1
		end
	end

	Element("ScrollingFrame", {
		id = "panel",
		Position = UDim2.new(1, -356, 0, 16),
		Size = UDim2.new(0, 340, 1, -32),
		tags = { "bg-none", "flex-col", "gap-sm", "pr-md" },
		ScrollingDirection = Enum.ScrollingDirection.Y,
		AutomaticCanvasSize = Enum.AutomaticSize.Y,
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = if state.finished
				then "Merge complete — file written"
				else `rbx-diff: {#state.conflicts - unresolved}/{#state.conflicts} decided`,
			Size = UDim2.new(1, 0, 0, 26),
			tags = { "corner-md" },
			BackgroundColor3 = Color3.fromRGB(28, 28, 32),
			TextColor3 = if state.finished then Color3.fromRGB(120, 220, 140) else Color3.fromRGB(230, 230, 230),
			TextSize = 14,
			Font = Enum.Font.GothamMedium,
		})

		if state.finished then
			return
		end

		for _, conflict in state.conflicts do
			ConflictRow(conflict)
		end

		local canComplete = unresolved == 0 and not state.completing
		Element("TextButton", {
			id = "complete",
			Text = if state.completing
				then "Completing…"
				elseif state.completeError then "Complete failed — retry"
				elseif unresolved > 0 then `Complete ({unresolved} undecided)`
				else "Complete",
			Size = UDim2.new(1, 0, 0, 30),
			tags = { "corner-md" },
			BackgroundColor3 = if canComplete then Color3.fromRGB(60, 160, 90) else Color3.fromRGB(55, 55, 60),
			TextColor3 = Color3.fromRGB(240, 240, 240),
			TextSize = 14,
			Font = Enum.Font.GothamMedium,
			events = {
				Activated = function()
					if canComplete then
						transport.complete()
					end
				end,
			},
		})
	end)
end

return Panel
