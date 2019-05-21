import 'package:flutter_web/material.dart';

import 'package:pixel_flutter_web/style/text.dart';
import 'package:pixel_flutter_web/style/colors.dart';

class SubmitButton extends StatelessWidget {
  final double width;
  final String text;
  final Function onPressed;
  final ButtonState buttonState;

  SubmitButton(
      {@required this.width,
      @required this.text,
      @required this.buttonState,
      @required this.onPressed});

  @override
  Widget build(BuildContext context) {
    return Container(
        width: width,
        height: 30,
        child: RaisedButton(
            clipBehavior: Clip.antiAlias,
            shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(10.0)),
            disabledColor: Colors.black12,
            color: primaryColor,
            onPressed: onPressed,
            child: buttonState == ButtonState.Reset
                ? Text(text, style: submitButtonStyle)
                : buttonState == ButtonState.Loading
                    ? SizedBox(
                        height: 20,
                        width: 20,
                        child: CircularProgressIndicator(
                          value: null,
                          valueColor:
                              AlwaysStoppedAnimation<Color>(Colors.white),
                          strokeWidth: 2,
                        ))
                    : SizedBox(
                        height: 20,
                        width: 20,
                        child: Icon(Icons.check, color: Colors.white),
                      )));
  }
}

enum ButtonState {
  Loading,
  Success,
  Reset,
}
