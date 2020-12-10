use crate::errors::AppError;

pub fn print_zsh_setup(use_fzf: bool, use_skim: bool) -> Result<(), AppError> {
  let fw_completion = include_str!("setup.zsh");
  let basic_workon = include_str!("workon.zsh");
  let fzf_workon = include_str!("workon-fzf.zsh");
  let skim_workon = include_str!("workon-sk.zsh");
  println!("{}", fw_completion);
  if use_fzf {
    println!("{}", fzf_workon);
  } else if use_skim {
    println!("{}", skim_workon);
  } else {
    println!("{}", basic_workon);
  }
  Ok(())
}

pub fn print_bash_setup(use_fzf: bool, use_skim: bool) -> Result<(), AppError> {
  let setup = include_str!("setup.bash");
  let basic = include_str!("workon.bash");
  let fzf = include_str!("workon-fzf.bash");
  let skim = include_str!("workon-sk.bash");

  println!("{}", setup);
  if use_fzf {
    println!("{}", fzf);
  } else if use_skim {
    println!("{}", skim)
  } else {
    println!("{}", basic);
  }

  Ok(())
}

pub fn print_fish_setup(use_fzf: bool, use_skim: bool) -> Result<(), AppError> {
  let setup = include_str!("setup.fish");
  let basic = include_str!("workon.fish");
  let fzf = include_str!("workon-fzf.fish");
  let skim = include_str!("workon-sk.fish");

  println!("{}", setup);
  if use_fzf {
    println!("{}", fzf);
  } else if use_skim {
    println!("{}", skim);
  } else {
    println!("{}", basic);
  }

  Ok(())
}
