VERSION 5.00
Begin VB.Form frmOverride 
   BorderStyle     =   1  'Fixed Single
   Caption         =   "Override settings"
   ClientHeight    =   2505
   ClientLeft      =   45
   ClientTop       =   330
   ClientWidth     =   4680
   KeyPreview      =   -1  'True
   LinkTopic       =   "Form1"
   MaxButton       =   0   'False
   ScaleHeight     =   2505
   ScaleWidth      =   4680
   StartUpPosition =   1  'CenterOwner
   Begin VB.CommandButton cmdOkay 
      Caption         =   "&OK"
      Height          =   375
      Left            =   3240
      TabIndex        =   6
      Top             =   2040
      Width           =   1335
   End
   Begin VB.Frame fmeOverride 
      Caption         =   "Override:"
      Height          =   1815
      Left            =   120
      TabIndex        =   7
      Top             =   120
      Width           =   4455
      Begin VB.CheckBox chkDropPIM 
         Caption         =   "Drop default PIM to 92 (sha512/Whirlpool usage only)"
         Height          =   315
         Left            =   120
         TabIndex        =   5
         ToolTipText     =   "Note: For VeraCrypt with sha512/Whirlpool usage only"
         Top             =   1440
         Width           =   4215
      End
      Begin VB.CheckBox chkLimitPIM 
         Caption         =   "Limit PIM usage to passwords that are 64 characters"
         Height          =   315
         Left            =   120
         TabIndex        =   4
         Top             =   1200
         Width           =   4215
      End
      Begin VB.CheckBox chkRandom 
         Caption         =   "Make generation slightly more random"
         Height          =   315
         Left            =   120
         TabIndex        =   3
         ToolTipText     =   "Note: This could be a good thing or bad thing depending on randomness"
         Top             =   960
         Width           =   4215
      End
      Begin VB.CheckBox chk512 
         Caption         =   "Allow passwords up to 512 character(s)"
         Height          =   315
         Left            =   120
         TabIndex        =   2
         Top             =   720
         Width           =   4215
      End
      Begin VB.CheckBox chk256 
         Caption         =   "Allow passwords up to 256 character(s)"
         Height          =   315
         Left            =   120
         TabIndex        =   1
         Top             =   480
         Width           =   4215
      End
      Begin VB.CheckBox chk128 
         Caption         =   "Allow passwords up to 128 character(s)"
         Height          =   315
         Left            =   120
         TabIndex        =   0
         Top             =   240
         Width           =   4215
      End
   End
End
Attribute VB_Name = "frmOverride"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Private Sub chk128_Click()
  If chk256.Value = 0 And chk512.Value = 0 Then
    frmPassGen.maxPassLen = 128
  Else
    If chk128.Value = 1 Then MsgBox "Cannot set as already overridden to 256 or 512 characters checked.", vbExclamation, "Error"
    chk128.Value = 0
  End If
End Sub

Private Sub chk256_Click()
  If chk128.Value = 0 And chk512.Value = 0 Then
    frmPassGen.maxPassLen = 256
  Else
    If chk256.Value = 1 Then MsgBox "Cannot set as already overridden to 128 or 512 characters checked.", vbExclamation, "Error"
    chk256.Value = 0
  End If
End Sub

Private Sub chk512_Click()
  If chk128.Value = 0 And chk256.Value = 0 Then
    frmPassGen.maxPassLen = 512
  Else
    If chk512.Value = 1 Then MsgBox "Cannot set as already overridden to 128 or 256 characters checked.", vbExclamation, "Error"
    chk512.Value = 0
  End If
End Sub

Private Sub chkDropPIM_Click()
  If chkDropPIM.Value = 1 Then
    frmPassGen.overridePIM = 92
  Else
    frmPassGen.overridePIM = frmPassGen.defaultPIM
  End If
End Sub

Private Sub chkLimitPIM_Click()
  If chkLimitPIM.Value = 1 Then
    frmPassGen.passLenPIM = 64
  Else
    frmPassGen.passLenPIM = frmPassGen.defaultPassLenPIM
  End If
End Sub

Private Sub chkRandom_Click()
  frmPassGen.moreRandomness = chkRandom.Value
End Sub

Private Sub cmdOkay_Click()
  Unload Me
End Sub

Private Sub Form_KeyPress(KeyAscii As Integer)
  If KeyAscii = vbKeyEscape Then Unload Me
End Sub

Private Sub Form_Load()
  chk128.Value = IIf(frmPassGen.maxPassLen = 128, 1, 0)
  chk256.Value = IIf(frmPassGen.maxPassLen = 256, 1, 0)
  chk512.Value = IIf(frmPassGen.maxPassLen = 512, 1, 0)
  chkRandom.Value = IIf(frmPassGen.moreRandomness = 1, 1, 0)
  chkLimitPIM.Value = IIf(frmPassGen.passLenPIM = 64, 1, 0)
  chkDropPIM.Value = IIf(frmPassGen.overridePIM = 92, 1, 0)
End Sub

Private Sub Form_Terminate()
  End
  ' `-> I doubt this has any use; but just incase...
End Sub

Private Sub Form_Unload(Cancel As Integer)
  If chk128.Value = 0 And chk256.Value = 0 And chk512.Value = 0 And frmPassGen.maxPassLen <> frmPassGen.defaultMaxPassLen Then frmPassGen.maxPassLen = frmPassGen.defaultMaxPassLen
  Call frmPassGen.addLenNumbers
End Sub

' EOF
