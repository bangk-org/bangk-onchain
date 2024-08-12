#!/usr/bin/env fish

set ROOT (jj root)
for f in (jj status --color=never | head -n -2 | tail -n +2 | awk '{print $2}')
    echo $f
    if test (path extension $f) = ".rs"
        ~/.cargo/bin/auto-header --path "$ROOT/$f"
    end
end
