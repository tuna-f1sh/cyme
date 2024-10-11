set -e

function cargo_project_name() {
  grep -m 1 name Cargo.toml | cut -d '"' -f 2
}

bin_name=$(cargo_project_name)
# with remaining args
cargo install --locked "$@" --path .

# supporting stuff
if [[ "$OSTYPE" == "darwin"* ]]; then
  if [[ -d /usr/local/etc/bash_completion.d ]]; then
    cp -v ./doc/"${bin_name}".bash /usr/local/etc/bash_completion.d/
  fi
  if [[ -d /usr/local/share/zsh/site-functions ]]; then
    cp -v ./doc/_"${bin_name}" /usr/local/share/zsh/site-functions/
  fi
  cp -v ./doc/"${bin_name}".1 /usr/local/share/man/man1/
elif [[ "$OSTYPE" == "linux"* ]]; then
  if [[ -d /usr/share/bash-completion/completions ]]; then
    cp -v ./doc/"${bin_name}".bash /usr/share/bash-completion/completions/
  fi
  if [[ -d /usr/share/zsh/site-functions ]]; then
    cp -v ./doc/_"${bin_name}" /usr/share/zsh/site-functions/
  fi
  # install man page
  cp -v ./doc/"${bin_name}".1 /usr/share/man/man1/
fi
