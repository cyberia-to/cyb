$env.config = {
    show_banner: false
    edit_mode: emacs
    cursor_shape: {
        emacs: line
        vi_insert: line
        vi_normal: block
    }
    table: {
        mode: rounded
        index_mode: auto
        trim: {
            methodology: wrapping
            wrapping_try_keep_words: true
        }
    }
    completions: {
        case_sensitive: false
        quick: true
        partial: true
        algorithm: "prefix"
    }
    history: {
        max_size: 10000
        sync_on_enter: true
        file_format: "sqlite"
    }
}
