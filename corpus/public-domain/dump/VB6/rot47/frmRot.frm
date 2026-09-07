VERSION 5.00
Begin VB.Form frmRot 
   BorderStyle     =   1  'Fixed Single
   Caption         =   "Rot47"
   ClientHeight    =   4920
   ClientLeft      =   45
   ClientTop       =   615
   ClientWidth     =   5760
   KeyPreview      =   -1  'True
   LinkTopic       =   "Form1"
   MaxButton       =   0   'False
   ScaleHeight     =   4920
   ScaleWidth      =   5760
   StartUpPosition =   2  'CenterScreen
   Begin VB.Timer tmrNoteClear 
      Enabled         =   0   'False
      Interval        =   3000
      Left            =   3840
      Top             =   4440
   End
   Begin VB.CommandButton cmdCopy 
      Caption         =   "C&opy"
      Height          =   375
      Left            =   120
      TabIndex        =   3
      Top             =   4440
      Width           =   1335
   End
   Begin VB.CommandButton CmdClose 
      Caption         =   "&Close"
      Height          =   375
      Left            =   4320
      TabIndex        =   2
      Top             =   4440
      Width           =   1335
   End
   Begin VB.TextBox TxtOutput 
      BackColor       =   &H80000004&
      Height          =   2055
      Left            =   120
      Locked          =   -1  'True
      MultiLine       =   -1  'True
      ScrollBars      =   2  'Vertical
      TabIndex        =   1
      Top             =   2280
      Width           =   5535
   End
   Begin VB.TextBox TxtInput 
      Height          =   2055
      Left            =   120
      MultiLine       =   -1  'True
      ScrollBars      =   2  'Vertical
      TabIndex        =   0
      Top             =   120
      Width           =   5535
   End
   Begin VB.Menu menuFile 
      Caption         =   "&File"
      Begin VB.Menu menuExit 
         Caption         =   "&Exit"
         Shortcut        =   ^E
      End
   End
End
Attribute VB_Name = "frmRot"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Private Sub CmdClose_Click()
  End
End Sub

Private Sub cmdCopy_Click()
  On Error GoTo endCopy
  Clipboard.Clear
  Clipboard.SetText TxtOutput.Text
  Me.Caption = Me.Caption & " - Copied to clipboard"
  tmrNoteClear.Enabled = True
  Exit Sub

endCopy:
  MsgBox "Failed to copy to clipboard.", vbExclamation, "Error"
End Sub

Private Sub Form_KeyPress(KeyAscii As Integer)
  If KeyAscii = vbKeyEscape Then End
End Sub

Private Sub Form_Load()
  Me.Tag = Me.Caption
End Sub

Private Sub Form_Terminate()
  End
  ' `-> I doubt this has any use; but just incase...
End Sub

Private Sub Form_Unload(Cancel As Integer)
  End
  ' `-> I doubt this has any use; but just incase...
End Sub

Private Sub menuExit_Click()
  End
End Sub

Private Sub tmrNoteClear_Timer()
  Me.Caption = Me.Tag
  tmrNoteClear.Enabled = False
End Sub

Private Sub TxtInput_Change()
  TxtOutput.Text = rotCode(TxtInput.Text)
End Sub

Function rotCode(inputString As String) As String
  Dim Code As String, I As Integer
  For I = 1 To Len(inputString)
    Dim Ascii As Integer
    Ascii = Asc(Mid(inputString, I, 1))
    If Ascii > 32 And Ascii < 127 Then
      Code = Code & Chr(Val(33 + (Ascii + 14) Mod 94))
    Else
      ' ,-> Ignore non-47 chars.
      Code = Code & Chr(Ascii)
    End If
  Next
  rotCode = RTrim(Code)
End Function

' EOF
