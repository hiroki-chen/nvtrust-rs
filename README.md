# nvtrust-rs

<s>This is a Rust port of `nvtrust` library which is written in Python. It is still under construction.</s>

Not only a nvtrust!

Note that this crate only supports H100 series.

```shell
Usage: nvtrust [OPTIONS]

Options:
      --gpu <GPU>                   Select the index of the GPU. [default: -1]
      --gpu-bdf <GPU_BDF>           Select a single GPU by providing a substring of the BDF, e.g. '01:00'.
      --gpu-name <GPU_NAME>         Select a single GPU by providing a substring of the GPU name, e.g. 'T4'. If multiple GPUs match, the first one will be used.
      --no-gpu                      Do not use any of the GPUs; commands requiring one will not work.
      --log <LOG>                   [default: info]
      --reset-with-os               Reset with OS through /sys/.../reset
      --query-cc-mode               Query the current Confidential Computing (CC) mode of the GPU.
      --query-cc-settings           Query the current Confidential Computing (CC) settings of the GPU.
                                    This prints the lower level setting knobs that will take effect upon GPU reset.
      --set-cc-mode <SET_CC_MODE>   Configure Confidentail Computing (CC) mode. The choices are off (disabled), on (enabled) or devtools (enabled in DevTools mode).
                                    
                                            The GPU needs to be reset to make the selected mode active. See --reset-after-cc-mode-switch for one way of doing it. [possible values: off, on, dev-tools]
      --reset-after-cc-mode-switch  Reset the GPU after switching CC mode such that it is activated immediately.
  -h, --help                        Print help (see more with '--help')
  -V, --version                     Print version
```

# Disclaimer

This tool is not endorsed by NVIDIA and is not NVIDIA's official tool!
