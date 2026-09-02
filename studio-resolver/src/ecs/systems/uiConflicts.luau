--[[
	Conflicts dock widget: the conflict list + Complete, rendered
	immediate-mode every heartbeat.
]]

local packages = require("../../packages")
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local theme = require("../../library/theme")

local types = require("../types")
local widgets = require("./library/widgets")

local Panel = require("../../ui/Panel")

return function(_world: types.World, _cts: types.Cts, scheduler: types.Scheduler)
	local widget = widgets.create("GitRbxMergeConflicts", "rbx-diff — conflicts", Enum.InitialDockState.Right, {
		width = 460,
		height = 600,
		minWidth = 320,
		minHeight = 260,
	})

	scheduler:addSystem({
		name = "ui_conflicts",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("rbxdiff_ui_conflicts")
			Portal(widget, "rbx-diff-conflicts", function()
				debug.profilebegin("rbxdiff_conflicts_body")
				theme.link()
				Element("Frame", {
					id = "conflicts",
					Size = UDim2.fromScale(1, 1),
					tags = { "surface", "vs-border" },
				}, function()
					Panel()
				end)
				debug.profileend()
			end)
			debug.profileend()
		end,
	})
end
