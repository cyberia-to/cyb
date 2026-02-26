$env.PROMPT_COMMAND = {||
    let dir = ($env.PWD | path basename)
    let home = $env.HOME
    let display_path = if ($env.PWD == $home) { "~" } else { $dir }

    let git_branch = (do -i { git branch --show-current } | complete | get stdout | str trim)
    let git_dirty = (do -i { git status --porcelain } | complete | get stdout | str trim)

    let dir_part = $"(ansi cyan_bold)($display_path)(ansi reset)"
    let git_part = if ($git_branch != "") {
        let dirty_marker = if ($git_dirty != "") { $"(ansi red)*" } else { "" }
        $" (ansi magenta)($git_branch)($dirty_marker)(ansi reset)"
    } else { "" }

    $"($dir_part)($git_part)"
}

$env.PROMPT_INDICATOR = {|| $"(ansi green_bold) ❯(ansi reset) " }
$env.PROMPT_COMMAND_RIGHT = ""
$env.PROMPT_MULTILINE_INDICATOR = {|| $"(ansi dark_gray):::(ansi reset) " }
