VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Form1"
   ClientHeight    =   6255
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   6660
   LinkTopic       =   "Form1"
   ScaleHeight     =   6255
   ScaleWidth      =   6660
   StartUpPosition =   3  'Windows Default
   Begin VB.CommandButton Command1 
      Caption         =   "&Draw"
      Height          =   465
      Left            =   2265
      TabIndex        =   1
      Top             =   5610
      Width           =   1950
   End
   Begin VB.PictureBox Picture1 
      Height          =   5070
      Left            =   675
      ScaleHeight     =   5010
      ScaleWidth      =   5310
      TabIndex        =   0
      Top             =   300
      Width           =   5370
   End
   Begin VB.Label Label1 
      Caption         =   "Label1"
      Height          =   975
      Left            =   315
      TabIndex        =   2
      Top             =   5535
      Width           =   1155
   End
End
Attribute VB_Name = "Form1"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Private Sub Command1_Click()
Dim startcolor As COLORREF
Dim endcolor As COLORREF
endcolor.lBlue = 255
CreateGradient Label1, startcolor, endcolor

End Sub

