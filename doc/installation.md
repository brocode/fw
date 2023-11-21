# Installation

The best way to install fw is the rust tool cargo.

``` bash
cargo install fw
```

If you are using OSX, [rustup](https://rustup.rs/) is recommended but
you [should be able to use brew
too](https://github.com/Homebrew/homebrew-core/pull/14490).

If you\'re lucky enough to be an arch linux user:
[AUR](https://aur.archlinux.org/packages/fw/)

If you are running on Windows then you will have some issue compiling
openssl. Please refer to compiling with rust-openssl
[here](https://github.com/sfackler/rust-openssl/blob/5948898e54882c0bedd12d87569eb4dbee5bbca7/README.md#windows-msvc)

## With fzf

Since we integrate with [fzf](https://github.com/junegunn/fzf) it is
recommended to use that or [skim](https://github.com/lotabout/skim) for
the best possible experience (`workon` and `nworkon` will be helm-style
fuzzy finders). Make sure `fzf` is installed and then add this to your
shell configuration:

Zsh (This shell is used by the project maintainers. The support for
other shells is untested by us):

``` shell-script
if [[ -x "$(command -v fw)" ]]; then
  if [[ -x "$(command -v fzf)" ]]; then
    eval $(fw print-zsh-setup -f 2>/dev/null);
  else
    eval $(fw print-zsh-setup 2>/dev/null);
  fi;
fi;
```

Bash:

``` shell-script
if [[ -x "$(command -v fw)" ]]; then
  if [[ -x "$(command -v fzf)" ]]; then
    eval "$(fw print-bash-setup -f 2>/dev/null)"
  else
    eval "$(fw print-bash-setup 2>/dev/null)"
  fi
fi
```

Fish:

``` shell-script
if test -x (command -v fw)
  if test -x (command -v fzf)
    fw print-fish-setup -f | source
  else
    fw print-fish-setup | source
  end
end
```

## With skim

We also integrate with [skim](https://github.com/lotabout/skim), you can
use that instead of fzf for the best possible experience (`workon` and
`nworkon` will be helm-style fuzzy finders).

If you have cargo installed you can install skim like this:

``` shell-script
cargo install skim
```

Make sure `skim` is installed and then add this to your shell
configuration:

Zsh (This shell is used by the project maintainers. The support for
other shells is untested by us):

``` shell-script
if [[ -x "$(command -v fw)" ]]; then
  if [[ -x "$(command -v sk)" ]]; then
    eval $(fw print-zsh-setup -s 2>/dev/null);
  else
    eval $(fw print-zsh-setup 2>/dev/null);
  fi;
fi;
```

Bash:

``` shell-script
if [[ -x "$(command -v fw)" ]]; then
  if [[ -x "$(command -v sk)" ]]; then
    eval "$(fw print-bash-setup -s 2>/dev/null)"
  else
    eval "$(fw print-bash-setup 2>/dev/null)"
  fi
fi
```

Fish:

``` shell-script
if test -x (command -v fw)
  if test -x (command -v sk)
    fw print-fish-setup -s | source
  else
    fw print-fish-setup | source
  end
end
```

## Without fzf or skim

If you don\'t want `fzf` or `skim` integration:

Zsh (This shell is used by the project maintainers. The support for
other shells is untested by us):

``` shell-script
if [[ -x "$(command -v fw)" ]]; then
  eval $(fw print-zsh-setup 2>/dev/null);
fi;
```

Bash:

``` shell-script
[[ -x "$(command -v fw)" ]] && eval "$(fw print-bash-setup)"
```

Fish:

``` shell-script
test -x (command -v fw) && fw print-fish-setup | source
```

In this case, `workon` and `nworkon` will require an argument (the
project) and will provide simple prefix-based autocompletion. You should
really use the `fzf` or `skim` integration though, it\'s much better!
