VERSION 5.00
Begin VB.Form FrmAsc 
   BorderStyle     =   1  'Fixed Single
   Caption         =   "ASCII Translator"
   ClientHeight    =   3720
   ClientLeft      =   45
   ClientTop       =   330
   ClientWidth     =   4680
   LinkTopic       =   "Form1"
   MaxButton       =   0   'False
   ScaleHeight     =   3720
   ScaleWidth      =   4680
   StartUpPosition =   2  'CenterScreen
   Begin VB.CommandButton CmdClose 
      Caption         =   "&Close"
      Height          =   375
      Left            =   3600
      TabIndex        =   3
      Top             =   3240
      Width           =   975
   End
   Begin VB.CheckBox ChkBreak 
      Caption         =   "&Break up output"
      Height          =   330
      Left            =   120
      TabIndex        =   2
      Top             =   3275
      Width           =   2295
   End
   Begin VB.TextBox TxtOutput 
      BackColor       =   &H80000004&
      Height          =   1455
      Left            =   120
      Locked          =   -1  'True
      MultiLine       =   -1  'True
      ScrollBars      =   2  'Vertical
      TabIndex        =   1
      Top             =   1680
      Width           =   4455
   End
   Begin VB.TextBox TxtInput 
      Height          =   1455
      Left            =   120
      MultiLine       =   -1  'True
      ScrollBars      =   2  'Vertical
      TabIndex        =   0
      Top             =   120
      Width           =   4455
   End
End
Attribute VB_Name = "FrmAsc"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Private Sub ChkBreak_Click()
  If TxtInput.Text <> "" Then
    TxtOutput.Text = AscCode(TxtInput.Text)
  End If
End Sub

Private Sub CmdClose_Click()
  End
End Sub

Private Sub Form_Terminate()
  End
  ' `-> I doubt this has any use; but just incase...
End Sub

Private Sub Form_Unload(Cancel As Integer)
  End
  ' `-> I doubt this has any use; but just incase...
End Sub

Private Sub TxtInput_Change()
  TxtOutput.Text = AscCode(TxtInput.Text)
End Sub

Function AscCode(AscString As String) As String
  Dim Code As String, I As Integer
  For I = 1 To Len(AscString)
    Code = Code & Asc(Mid(AscString, I, 1))
    If ChkBreak.Value = 1 Then
      Code = Code & " "
    End If
  Next
  AscCode = RTrim(Code)
End Function

' EOF
