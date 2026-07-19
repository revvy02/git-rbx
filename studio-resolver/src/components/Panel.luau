local Element = require("../packages").miu.Instance

local state = require("../state")
local theme = require("../theme")
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
		Size = UDim2.new(1, 0, 1, 0),
		tags = { "bg-none", "flex-col" },
		ScrollingDirection = Enum.ScrollingDirection.Y,
		AutomaticCanvasSize = Enum.AutomaticSize.Y,
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = if state.finished
				then "MERGE COMPLETE — FILE WRITTEN"
				else `CONFLICTS — {#state.conflicts - unresolved}/{#state.conflicts} DECIDED`,
			Size = UDim2.new(1, 0, 0, 24),
			BackgroundColor3 = theme.surfaceHigh,
			TextColor3 = if state.finished then theme.added else theme.textMuted,
			TextSize = theme.headerSize,
			Font = theme.font,
		})

		if state.finished then
			return
		end

		Element("Frame", {
			id = "content",
			tags = { "bg-none", "auto-size-y", "flex-col", "gap-sm", "p-sm" },
		}, function()
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
				Size = UDim2.new(1, 0, 0, 28),
				tags = { "corner-sm" },
				BackgroundColor3 = if canComplete then theme.accent else theme.disabled,
				TextColor3 = if canComplete then theme.textBright else theme.textMuted,
				TextSize = theme.textSize,
				Font = theme.font,
				events = {
					Activated = function()
						if canComplete then
							transport.complete()
						end
					end,
				},
			})
		end)
	end)
end

return Panel
