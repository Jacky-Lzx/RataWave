-- stylua: ignore
require("dap").configurations.rust = {
  {
    name = "Launch file in external terminal",
    type = "codelldb",
    request = "launch",
    program = function()
      -- Build the project before starting a debug session
      vim.fn.system("cargo build")

      -- Get the file name of the target executable
      local metadata_json = vim.fn.system("cargo metadata --format-version 1 --no-deps")
      local metadata = vim.fn.json_decode(metadata_json)
      local target_name = metadata.packages[1].targets[1].name
      local target_dir = metadata.target_directory
      return target_dir .. "/debug/" .. target_name
    end,
    args = function()
      -- Command line arguments that will be passed to the program
      local inputstr = vim.fn.input("CommandLine args: ", "")
      local params = {}
      for param in string.gmatch(inputstr, "[^%s]+") do
        table.insert(params, param)
      end
      return params
    end,
    console = "externalTerminal",
  },
}
