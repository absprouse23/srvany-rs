# srvany-rs (WIP)
A drop in replacement for [srvany-ng](https://github.com/birkett/srvany-ng), written in Rust.

## Installing
Place srvany-rs in an accessible folder on your system.
Install it as a service from an Elevated (Administrator) Command Prompt:
```winbatch
sc create "MyServiceName" start= auto binPath= "C:\Path\To\srvany-rs.exe"
sc description MyServiceName "My services description"
```
Note the spaces between `start=`, `binPath=` and their parameters. This is intended.

Now, open the Registry editor (`regedit.exe`), and browse to:
`HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\MyServiceName`

Create a new Key named "Parameters".  
In the parameters key, create a new String value named "Application". The value should be the file path to the application you wish to run as a service.

#### Optional Parameters
| Value name     | Value type         | Description                                                                                       |
|----------------|--------------------|---------------------------------------------------------------------------------------------------|
| AppDirectory   | String value       | The starting directory for your application. Usually the same as the folder its executable is in. |
| AppParameters  | String value       | Command line arguments to pass to your application on startup.                                    |
| AppEnvironment | Multi-String value | Environment variables to set for your application.                                                |
| RestartOnExit  | DWORD value        | If set to 1, and the application exits, srvany-ng will automatically restart it.                  |

## Further Reading
Microsoft support article describing the use of the original srvany.exe: https://support.microsoft.com/en-us/kb/137890

## srvany-ng
srvany-rs is designed to be a drop in replacement for the C-based srvany-ng tool. A link to the original project by Anthony Birkett can be found [here](https://github.com/birkett/srvany-ng).