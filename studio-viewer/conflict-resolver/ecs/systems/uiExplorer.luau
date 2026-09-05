-- Explorer dock widget, rendered immediate-mode every heartbeat.

local packages = require("@shared/packages")
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local theme = require("@shared/library/theme")

local types = require("../types")
local widgets = require("@shared/widgets")

local ExplorerPanel = require("../../ui/ExplorerPanel")

return function(_world: types.World, _cts: types.Cts, scheduler: types.Scheduler)
	local widget = widgets.create("GitRbxMergeExplorer", "git-rbx — explorer", Enum.InitialDockState.Left, {
		width = 360,
		height = 600,
		minWidth = 260,
		minHeight = 240,
	})

	scheduler:addSystem({
		name = "ui_explorer",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("gitrbx_ui_explorer")
			Portal(widget, "git-rbx-explorer", function()
				theme.link()
				Element("Frame", {
					id = "explorer-widget",
					Size = UDim2.fromScale(1, 1),
					tags = { "surface", "vs-border" },
				}, function()
					ExplorerPanel()
				end)
			end)
			debug.profileend()
		end,
	})
end
