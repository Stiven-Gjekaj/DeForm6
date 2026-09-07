**********************************
This useful module was created
by Drew Burchett in response
to a question I had with a sub
that I was having trouble writing. 
You can check out his site at;
**********************************
Drew's Development Environment
http://www.ddesoftware.com/
**********************************
To test this sample you'll need to
change the Command1_Click event, where
it says Label1 to Picture1. This code 
should work on any control that has a
hWnd property. Picture box does the 
label doesn't.


Private Sub Command1_Click()
  Dim startcolor As COLORREF
  Dim endcolor As COLORREF
  endcolor.lBlue = 255
  CreateGradient Picture1, startcolor, endcolor
End Sub

**********************************
Burt Abreu  08/24/98
Visual Basic Explorer
http://www.vbexplorer.com
**********************************