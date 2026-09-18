# Windows reader troubleshooting

Follow these steps when the DigitalPersona U.are.U 4500 reader is connected but
is not detected by the plugin on Windows.

The plugin communicates with the reader through WinUSB. It does not require the
DigitalPersona SDK or proprietary runtime, but Windows must associate the reader
with its built-in WinUSB driver.

> **Warning:** selecting the wrong USB device in Zadig can replace the driver for
> another device. Confirm that the selected device is the U.are.U 4500 reader
> before replacing its driver.

## Configure the reader with Zadig

1. Download [Zadig from its official website](https://zadig.akeo.ie/) and run
   it as Administrator. Zadig is portable, so it does not need to be installed.
2. If the reader does not appear in the device list, open the **Options** menu
   and enable **List All Devices**.
3. Select **U.are.U 4500 Fingerprint Reader** from the device list. Verify that
   its USB ID is `05BA:000A` before continuing.
4. Select **WinUSB** as the target driver, click **Replace Driver**, and wait
   until the process finishes successfully.
5. Disconnect the reader, reconnect it, and restart the Tauri application.

After reconnecting the reader, call `checkDevice()` again. A successful result
returns `detected: true`.

```ts
import { checkDevice } from 'tauri-plugin-fingerprint'

const status = await checkDevice()
console.log(status.detected, status.message)
```
