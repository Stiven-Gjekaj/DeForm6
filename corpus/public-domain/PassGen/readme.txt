PassGen v1.2.2 by Jigsy (https://github.com/Jigsy1) released under The Unlicense.

Compatible with:
  * Windows 98 SE (requires MSVBVM60.dll)
  * Windows ME
  * Windows XP
  * Windows 8.1

Assumed compatibility with:
  * Windows Vista
  * Windows 7
  * Windows 8
  * Windows 10
  * Windows 11

Not compatible with:
  * Windows 3.1 (assumed)
  * Windows 95 (assumed)
  * WINE <=1.8
    * Unsure about other versions >1.8

How to use:
---------------
Simply run the PassGen.exe file.

Upon loading you will be presented with a form that includes certain options such as characters to include, number of passwords, etc.

The default options for passwords are to include Uppercase characters, Lowercase characters, Numbers and Special characters.
  For a total of 64 passwords, each 16 characters long.


To generate a list of password(s), simply press "Generate." (Or press F5.)
  You can generate a minimum of 1 password, and a maximum of 1,024.

If you wish to clear the list from prying eyes, press "Del." (Located next to the Ins key at the top of the keyboard.)


Clicking the [?] next to "Include Special characters..." will bring up a form allowing you to choose what special characters you
  want to include.
Deselecting all of them will disable special characters until you choose at least one character.
  There is a button to include everything if you've deselected everything prior.

You can also access this from the menubar. (CTRL+I)


Password length is limited from 8 to 64 characters. However, you can override this by selecting from the Menubar->Settings->Override. (Or pressing CTRL+O.)
  This will bring up a form which allows you to increase password length to 128 characters, 256 or 512 characters.

There is also a drop down option for "Rand" characters, which will generate passwords of random length from 8 to <the maximum set>.


Also under the override form is an option to increase the randomness of the passwords generated if you do not feel the current randomness
  is sufficient.


Another four options on the main form are "Automatically generate new password(s) (60s)" which is pretty self-explainatory,
  "Avoid using the same characters in succession," which will prevent things like, for example, ABBA (in which B follows B),
  "Avoid using space on the first/last character(s)," which will prevent any passwords where Space is included from using space on the first and last character,
  and "Include a random PIM from 1 to <N>."
If you're a user of VeraCrypt, you can include a random number (a PIM) after your password for use in file(s) or volumes(s).
  This will appear as: <password> ------------ <N>

Note: If the pasword is less than 20 characters, and the PIM drop down is either set as 128 or 256, this will not appear.
      This is due to the fact that the default PIM in VeraCrypt is 485 - unless sha512 or Whirlpool are selected, in which case it is 98.
      For those who do use sha512/Whirlpool, there is an override option to drop the default PIM to 92.

If you wish to copy a password to the clipboard, simply select the password in the list and press "Copy."
  If you wish to copy all the passwords, press "Select All," which will automatically select everything for you, then press copy.

Note: Windows 8.1 has an issue copying a large number of large passwords to the clipboard, an issue which didn't occur on Windows XP. (Or 98 SE!)
        If this happens, you will be told that you failed to copy to the clipboard. The only fallback option I can suggest is copying a certain amount at
        a time.

      If you were successful in copying to the clipboard, the titlebar will change briefly.


If you wish to save your settings for next time click the "Save Settings to Registry on Exit" option in the menubar (under Settings), then close the application.
  To clear the settings, simply click the same option again.


Note: If this fails to work, open regedit, then delete the following key: HKCU\Software\Github\Jigsy1\PassGen\



Bug(s)/Etc.:
---------------
If you find a bug, please feel free to open an issue on Github.

If it's (not) compatible with a version of Windows/WINE, please let me know by opening an issue on Github.



-Jigsy
