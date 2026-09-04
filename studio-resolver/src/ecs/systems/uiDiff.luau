-- Direct conflict diff dock widget, synchronized through SelectedConflict.

local packages = require("../../packages")
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local theme = require("../../library/theme")

local types = require("../types")
local widgets = require("./library/widgets")

local ImpactPanel = require("../../ui/ImpactPanel")

return function(_world: types.World, _cts: types.Cts, scheduler: types.Scheduler)
	local widget = widgets.create("GitRbxMergeDirectDiff", "git-rbx — direct diff", Enum.InitialDockState.Right, {
		width = 500,
		height = 360,
		minWidth = 320,
		minHeight = 180,
	})

	scheduler:addSystem({
		name = "ui_diff",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("gitrbx_ui_diff")
			Portal(widget, "git-rbx-direct-diff", function()
				theme.link()
				Element("Frame", {
					id = "diff-widget",
					Size = UDim2.fromScale(1, 1),
					tags = { "surface", "vs-border" },
				}, function()
					ImpactPanel()
				end)
			end)
			debug.profileend()
		end,
	})
end
