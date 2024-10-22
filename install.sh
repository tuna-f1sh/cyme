set -e

function cargo_project_name() {
  grep -m 1 name Cargo.toml | cut -d '"' -f 2
}

bin_name=$(cargo_project_name)
# with remaining args
cargo install --locked "$@" --path .
install -D -m 0755 "${HOME}/.cargo/bin/${bin_name}" "${DESTDIR}/usr/bin/${bin_name}"

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
install -D -m 0644 ./doc/"${bin_name}".bash "${DESTDIR}/usr/share/bash-completion/completions/${bin_name}.bash"
install -D -m 0644 ./doc/_"${bin_name}" "${DESTDIR}/usr/share/zsh/site-functions/_${bin_name}"
install -D -m 0644 ./doc/"${bin_name}".1 "${DESTDIR}/usr/share/man/man1/${bin_name}.1"
fi
