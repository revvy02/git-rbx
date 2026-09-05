local Element = require("@shared/packages").miu.Element

local format = require("@shared/library/format")
local summary = require("../library/summary")
local theme = require("@shared/library/theme")

local conflicts = require("../ecs/systems/library/conflicts")
local cts = require("../ecs/cts")
local world = require("../ecs/world")

local Label = require("@shared/ui/Label")
local OptionRow = require("./OptionRow")

-- base | ours | theirs for a conflict entity; clicking a row stages it (base =
-- unpick). When custom is staged, shows the live value Complete will capture.
local function ComparisonCard(entity: any)
	local kind = world:get(entity, cts.Kind) :: string
	local path = world:get(entity, cts.Path) :: string
	local property = world:get(entity, cts.Property) :: string?
	local resolution = world:get(entity, cts.Resolution) :: string?
	local base = world:get(entity, cts.Base) :: cts.Base

	Element("Frame", {
		id = "comparison",
		tags = { "auto-size-y", "flex-col", "gap-sm", "p-md", "corner-sm", "surface", "vs-border" },
	}, function()
		Label("path", path, theme.statusColor(resolution))
		Element("TextLabel", {
			id = "kind",
			Text = kind .. (if property then ` ({property})` else ""),
			tags = { "text", "text-small", "muted", "auto-size-y" },
		})

		OptionRow("base", "base", summary.base(kind, base), theme.textMuted, resolution == nil, function()
			conflicts.stage(entity, nil)
		end)
		OptionRow(
			"ours",
			"ours",
			summary.side(kind, property, world:get(entity, cts.Ours)),
			theme.ours,
			resolution == "ours",
			function()
				conflicts.stage(entity, "ours")
			end
		)
		OptionRow(
			"theirs",
			"theirs",
			summary.side(kind, property, world:get(entity, cts.Theirs)),
			theme.theirs,
			resolution == "theirs",
			function()
				conflicts.stage(entity, "theirs")
			end
		)
		if resolution == "custom" then
			Label("custom", `custom — live: {format.formatValue(conflicts.liveValue(entity))}`, theme.custom)
		end
	end)
end

return ComparisonCard
