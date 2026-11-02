-- Axel: open the floating launcher/inbox from anywhere in Rex.
-- Append to ~/.config/rex/init.lua and run `rex config reload`.
rex.action{
  name = "axel",
  title = "Axel: launcher / inbox",
  description = "Open Axel as a floating centered pane in the current window",
  category = "Agents",
  run = function(ctx)
    rex.session.new_layer{
      session_id = ctx.session_id,
      bounds = {0.15, 0.10, 0.70, 0.80},
      focus = true,
      layout = { block = {
        flavor = "com.superlogical.terminal.shell",
        label = "axel",
        options = { command = {"axel", "ui"}, exit = { on_completion = true } },
      } },
    }
  end,
}
rex.bind("ctrl+a>a", "axel")
