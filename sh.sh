/usr/bin/podman run --rm -it --name am-clippy2 \
    --userns=keep-id:uid=1000,gid=1000 \
    -v /home/dstanek/src/github.com/dstanek/am/.am/worktrees/clippy:/home/dstanek/src/github.com/dstanek/am/.am/worktrees/clippy:rw,z \
    -v /home/dstanek/src/github.com/dstanek/am/.jj:/home/dstanek/src/github.com/dstanek/am/.jj:rw,z \
    -v /home/dstanek/src/github.com/dstanek/am/.git:/home/dstanek/src/github.com/dstanek/am/.git:rw,z \
    -v /home/dstanek/.gitconfig:/home/am/.gitconfig:ro,z \
    -v /home/dstanek/.ssh:/home/am/.ssh:ro,z \
    -v /home/dstanek/.claude:/home/am/.claude:rw,z \
    -v /home/dstanek/.claude.json:/home/am/.claude.json:rw,z \
    --workdir /home/dstanek/src/github.com/dstanek/am/.am/worktrees/clippy \
    localhost/am-claude
