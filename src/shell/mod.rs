use crate::errors::AppError;

pub fn print_zsh_setup(use_fzf: bool) -> Result<(), AppError> {
  let fw_completion = include_str!("setup.zsh");
  let basic_workon = include_str!("workon.zsh");
  let fzf_workon = include_str!("workon-fzf.zsh");
  println!("{}", fw_completion);
  if use_fzf {
    println!("{}", fzf_workon);
  } else {
    println!("{}", basic_workon);
  }
  Ok(())
}

pub fn print_bash_setup(use_fzf: bool) -> Result<(), AppError> {
  let setup = include_str!("setup.bash");
  let basic = include_str!("workon.bash");
  let fzf = include_str!("workon-fzf.bash");

  println!("{}", setup);
  if use_fzf {
    println!("{}", fzf);
  } else {
    println!("{}", basic);
  }

  Ok(())
}
