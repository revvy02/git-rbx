local packages = require("../packages")
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions

local theme = require("../library/theme")
local transport = require("../library/transport")

local conflicts = require("../ecs/systems/library/conflicts")
local cts = require("../ecs/cts")
local world = require("../ecs/world")

local ConflictRow = require("./ConflictRow")

local HEADER_HEIGHT = 24
local CARD_HEIGHT = 106
local CARD_GAP = 4
local FOOTER_HEIGHT = 36

-- The conflict list + Complete button. Cards render through VirtualList —
-- only the visible slice builds ConflictRows, so real merges (1000+
-- conflicts) stay interactive. Complete lives in a pinned footer instead of
-- after the last card.
local function Panel()
	local cards = conflicts.cards()
	debug.profilebegin("rbxdiff_panel_count")
	local unresolved = 0
	for _, entity in cards do
		local members = world:get(entity, cts.Members) :: { any }?
		local resolution = if members then conflicts.groupResolution(entity) else world:get(entity, cts.Resolution)
		if resolution == nil then
			unresolved += 1
		end
	end
	debug.profileend()

	local finished = world:has(cts.Finished, cts.Finished)
	local completing = world:has(cts.Completing, cts.Completing)
	local completeError = world:get(cts.CompleteError, cts.CompleteError) :: string?

	Element("Frame", {
		id = "panel",
		Size = UDim2.fromScale(1, 1),
		tags = { "bg-none", "flex-col" },
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = if finished
				then "MERGE COMPLETE — FILE WRITTEN"
				else `CONFLICTS — {#cards - unresolved}/{#cards} DECIDED`,
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
			TextColor3 = if finished then theme.added else theme.textMuted,
		})

		if finished then
			return
		end

		debug.profilebegin("rbxdiff_panel_cards")
		VirtualList({
			id = "cards",
			data = cards,
			dimensions = Dimensions.withSpacing(Dimensions.consistentSize(CARD_HEIGHT), CARD_GAP),
			direction = "y",
			Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT - FOOTER_HEIGHT),
			tags = { "bg-none" },
		}, function(entity: any)
			Element("UIPadding", {
				id = "pad",
				PaddingLeft = UDim.new(0, 4),
				PaddingRight = UDim.new(0, 4),
			})
			ConflictRow(entity)
		end)
		debug.profileend()

		Element("Frame", {
			id = "footer",
			Size = UDim2.new(1, 0, 0, FOOTER_HEIGHT),
			tags = { "bg-none", "flex-col", "p-sm" },
		}, function()
			local canComplete = unresolved == 0 and not completing
			Element("TextButton", {
				id = "complete",
				Text = if completing
					then "Completing…"
					elseif completeError then "Complete failed — retry"
					elseif unresolved > 0 then `Complete ({unresolved} undecided)`
					else "Complete",
				Size = UDim2.fromScale(1, 1),
				tags = { "corner-sm", "button" },
				BackgroundColor3 = if canComplete then theme.accent else theme.disabled,
				TextColor3 = if canComplete then theme.textBright else theme.textMuted,
				TextSize = theme.textSize,
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
