VERSION 5.00
Begin VB.Form frmUUID2 
   BorderStyle     =   1  'Fixed Single
   Caption         =   "UUID2"
   ClientHeight    =   1680
   ClientLeft      =   45
   ClientTop       =   615
   ClientWidth     =   9120
   KeyPreview      =   -1  'True
   LinkTopic       =   "Form1"
   MaxButton       =   0   'False
   ScaleHeight     =   1680
   ScaleWidth      =   9120
   StartUpPosition =   2  'CenterScreen
   Begin VB.Timer tmrAutomatic 
      Enabled         =   0   'False
      Interval        =   30000
      Left            =   7200
      Top             =   1200
   End
   Begin VB.Timer tmrNoteClear 
      Interval        =   2000
      Left            =   6720
      Top             =   1200
   End
   Begin VB.CommandButton cmdClose 
      Caption         =   "&Close"
      Height          =   375
      Left            =   7680
      TabIndex        =   8
      Top             =   1200
      Width           =   1335
   End
   Begin VB.CommandButton cmdCopy 
      Caption         =   "C&opy"
      Height          =   375
      Left            =   5280
      TabIndex        =   6
      Top             =   1200
      Width           =   1335
   End
   Begin VB.CommandButton cmdGenerate 
      Caption         =   "&Generate (F5)"
      Height          =   375
      Left            =   3840
      TabIndex        =   5
      Top             =   1200
      Width           =   1335
   End
   Begin VB.Frame fmeSettings 
      Height          =   1575
      Left            =   120
      TabIndex        =   9
      Top             =   0
      Width           =   3615
      Begin VB.CheckBox chkHyphens 
         Caption         =   "Include hyphens (...-...-...-...-...)"
         Height          =   195
         Left            =   120
         TabIndex        =   2
         Top             =   720
         Value           =   1  'Checked
         Width           =   2775
      End
      Begin VB.CheckBox chkRandomness 
         Caption         =   "Make generation slightly more random"
         Height          =   195
         Left            =   120
         TabIndex        =   4
         Top             =   1200
         Width           =   3135
      End
      Begin VB.CheckBox chkAutomatic 
         Caption         =   "Automatically generate new UUID2(s) (30s)"
         Height          =   195
         Left            =   120
         TabIndex        =   3
         Top             =   960
         Width           =   3375
      End
      Begin VB.CheckBox chkBraces 
         Caption         =   "Include {braces}"
         Height          =   195
         Left            =   120
         TabIndex        =   1
         Top             =   480
         Value           =   1  'Checked
         Width           =   2655
      End
      Begin VB.CheckBox chkUpper 
         Caption         =   "Use uppercase characters (A...Z)"
         Height          =   195
         Left            =   120
         TabIndex        =   0
         Top             =   240
         Value           =   1  'Checked
         Width           =   2775
      End
   End
   Begin VB.Frame fmeStyle 
      Height          =   735
      Left            =   3840
      TabIndex        =   10
      Top             =   0
      Width           =   5175
      Begin VB.TextBox txtUUID2 
         Alignment       =   2  'Center
         Height          =   375
         Left            =   120
         Locked          =   -1  'True
         TabIndex        =   7
         Top             =   240
         Width           =   4935
      End
   End
   Begin VB.Menu menuFile 
      Caption         =   "&File"
      Begin VB.Menu menuExit 
         Caption         =   "&Exit"
         Shortcut        =   ^E
      End
   End
   Begin VB.Menu menuSettings 
      Caption         =   "&Settings"
      Begin VB.Menu menuSave 
         Caption         =   "&Save Settings to Registry on Exit for Next Time"
      End
   End
   Begin VB.Menu menuAbout 
      Caption         =   "&About"
      Begin VB.Menu menuLicense 
         Caption         =   "&License"
         Shortcut        =   ^L
      End
      Begin VB.Menu menuSep 
         Caption         =   "-"
      End
      Begin VB.Menu menuWebsite 
         Caption         =   "&Website..."
         Shortcut        =   ^W
      End
   End
End
Attribute VB_Name = "frmUUID2"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Option Explicit

' ,-> Global variable(s).

Public ourPath As String
' `-> Registry entry.

' ,-> Code:

Public Function resetTimer()
  tmrAutomatic.Enabled = False
  tmrAutomatic.Enabled = True
End Function

Public Function isBool(inputBool As Variant)
  If LCase(inputBool) = "true" Or inputBool = "1" Then
    isBool = "1"
  Else
    ' `-> Treat everything else as false.
    isBool = "0"
  End If
End Function

Public Function isRegKey(inputKey As String)
  Dim thisKey As String
  Dim thisObject As Object
  Set thisObject = CreateObject("WScript.Shell")
  On Error Resume Next
  isRegKey = False
  thisKey = thisObject.RegRead(inputKey)
  If Err.Number = 0 Then
    isRegKey = True
  ElseIf thisKey = "" Then
    isRegKey = False
  ElseIf IsNull(thisKey) = True Then
    isRegKey = False
  ElseIf CBool(InStr(Err.Description, "Unable")) Then
    isRegKey = False
  End If
  Err.Clear: On Error GoTo 0
End Function

Public Function saveToRegistry()
  If menuSave.Checked = True Then
    Dim qS As Object
    Set qS = CreateObject("WScript.Shell")
    ' `-> q(uick)S(hell).
    ' qS.RegWrite ourPath & "useSave", useSave, "REG_DWORD"
    qS.RegWrite ourPath & "useUppercase", chkUpper.Value, "REG_DWORD"
    qS.RegWrite ourPath & "useBraces", chkBraces.Value, "REG_DWORD"
    qS.RegWrite ourPath & "useHyphens", chkHyphens.Value, "REG_DWORD"
    qS.RegWrite ourPath & "useAutomatic", chkAutomatic.Value, "REG_DWORD"
    qS.RegWrite ourPath & "moreRandomness", chkRandomness.Value, "REG_DWORD"
  End If
End Function

Private Sub chkAutomatic_Click()
  If chkAutomatic.Value = 1 Then
    tmrAutomatic.Enabled = True
  Else
    tmrAutomatic.Enabled = False
  End If
End Sub

Private Sub cmdClose_Click()
  Call saveToRegistry
  End
End Sub

Private Sub cmdCopy_Click()
  On Error GoTo endCopy
  Clipboard.Clear
  Clipboard.SetText txtUUID2.Text
  Me.Caption = Me.Caption & " - Copied to clipboard"
  tmrNoteClear.Enabled = True
  Exit Sub

endCopy:
  MsgBox "Failed to copy to clipboard.", vbExclamation, "Error"
End Sub

Private Sub cmdGenerate_Click()
  Call makeUUID2
  If tmrAutomatic.Enabled = True Then Call resetTimer
End Sub

Private Function makeUUID2() As String
  Dim baseString As String
  baseString = "abcdefghijklmnopqrstuvwxyz"
  If chkUpper.Value = 1 Then baseString = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
  baseString = baseString & "0123456789"
  Randomize
  Dim loopNumber As Integer, outString As String, randNumber As Integer
  For loopNumber = 0 To 32 - 1
    If chkRandomness.Value = 1 Then
      randNumber = Int(Val(1 + Val(Rnd * 6)))
      Select Case randNumber
        Case 1
          outString = outString & Mid(baseString, Int(Val(1 + Val(Rnd * Len(baseString)))), 1)
        Case 2
            outString = outString & Mid(StrReverse(baseString), Int(Val(1 + Val(Rnd * Len(StrReverse(baseString))))), 1)
        Case 3
          ' ,-> Former half
          outString = outString & Mid(Mid(baseString, 1, Val(Len(baseString) / 2)), Int(Val(1 + Val(Rnd * Len(Mid(baseString, 1, Val(Len(baseString) / 2)))))), 1)
        Case 4
          ' ,-> Latter half
          outString = outString & Mid(Mid(baseString, Val(Len(baseString) / 2)), Int(Val(1 + Val(Rnd * Len(Mid(baseString, Val(Len(baseString) / 2)))))), 1)
        Case 5
          ' ,-> Former half (Reverse)
          outString = outString & Mid(Mid(StrReverse(baseString), 1, Val(Len(StrReverse(baseString)) / 2)), Int(Val(1 + Val(Rnd * Len(Mid(StrReverse(baseString), 1, Val(Len(StrReverse(baseString)) / 2)))))), 1)
        Case 6
          ' ,-> Latter half (Reverse)
          outString = outString & Mid(Mid(StrReverse(baseString), Val(Len(StrReverse(baseString)) / 2)), Int(Val(1 + Val(Rnd * Len(Mid(StrReverse(baseString), Val(Len(StrReverse(baseString)) / 2)))))), 1)
      End Select
    Else
      outString = outString & Mid(baseString, Int(Val(1 + Val(Rnd * Len(baseString)))), 1)
    End If
    If loopNumber = 7 Or loopNumber = 11 Or loopNumber = 15 Or loopNumber = 19 Then
      If chkHyphens.Value = 1 Then outString = outString & "-"
    End If
  Next
  If chkBraces.Value = 1 Then outString = "{" & outString & "}"
  txtUUID2.Text = outString
  outString = ""
End Function

Private Sub Form_KeyDown(KeyCode As Integer, Shift As Integer)
  If KeyCode = vbKeyF5 Then
    Call makeUUID2
    If tmrAutomatic.Enabled = True Then Call resetTimer
  End If
End Sub

Private Sub Form_KeyPress(KeyAscii As Integer)
  If KeyAscii = vbKeyEscape Then
    Call saveToRegistry
    End
  End If
End Sub

Private Sub Form_Load()
  Me.Caption = Me.Caption & " v" & App.Major & "." & App.Minor & "." & App.Revision
  Me.Tag = Me.Caption
  ourPath = "HKCU\Software\Github\Jigsy1\UUID2\"
  ' `-> This is the registry key we're going to save to. If something goes wrong (because you changed something), just delete the entire key.
  If isRegKey(ourPath) = True Then
    menuSave.Checked = True
    Dim qS As Object
    ' `-> q(uick)S(hell)
    Set qS = CreateObject("WScript.Shell")
    ' If isRegKey(ourPath & "useSave") = True Then useSave = isBool(qS.RegRead(ourPath & "useSave"))
    If isRegKey(ourPath & "useUppercase") = True Then chkUpper.Value = isBool(qS.RegRead(ourPath & "useUppercase"))
    If isRegKey(ourPath & "useBraces") = True Then chkBraces.Value = isBool(qS.RegRead(ourPath & "useBraces"))
    If isRegKey(ourPath & "useHyphens") = True Then chkHyphens.Value = isBool(qS.RegRead(ourPath & "useHyphens"))
    If isRegKey(ourPath & "useAutomatic") = True Then chkAutomatic.Value = isBool(qS.RegRead(ourPath & "useAutomatic"))
    If isRegKey(ourPath & "moreRandomness") = True Then chkRandomness.Value = isBool(qS.RegRead(ourPath & "moreRandomness"))
  End If
  Call makeUUID2
End Sub

Private Sub Form_Terminate()
  Call saveToRegistry
  End
  ' `-> I doubt this has any use; but just incase...
End Sub

Private Sub Form_Unload(Cancel As Integer)
  Call saveToRegistry
  End
  ' `-> I doubt this has any use; but just incase...
End Sub

Private Sub menuExit_Click()
  Call saveToRegistry
  End
End Sub

Private Sub menuLicense_Click()
  frmAbout.Visible = True
End Sub

Private Sub menuSave_Click()
  If menuSave.Checked = True Then
    menuSave.Checked = False
    ' ,-> Destroy the registry entry. (It's easier!)
    Dim qS As Object
    Set qS = CreateObject("WScript.Shell")
    If isRegKey(ourPath) = True Then qS.RegDelete ourPath
    Me.Caption = Me.Tag & " - Cleared settings from the registry"
    tmrNoteClear.Enabled = True
  Else
    menuSave.Checked = True
  End If
End Sub

Private Sub menuWebsite_Click()
  CreateObject("WScript.Shell").Run frmAbout.lblLink.Caption
End Sub

Private Sub tmrAutomatic_Timer()
  Call makeUUID2
End Sub

Private Sub tmrNoteClear_Timer()
  Me.Caption = Me.Tag
  tmrNoteClear.Enabled = False
End Sub

' EOF
