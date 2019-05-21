import 'package:flutter_web/material.dart';

import 'package:pixel_flutter_web/components/SubmitButton/SubmitButton.dart';

class TopicSubmitButton extends StatefulWidget {
  final String type;
  final double width;
  final Function submit;
  final bool valid;

  TopicSubmitButton(
      {@required this.valid,
      @required this.type,
      @required this.width,
      @required this.submit});

  @override
  _TopicSubmitButtonState createState() => _TopicSubmitButtonState();
}

class _TopicSubmitButtonState extends State<TopicSubmitButton> {
  ButtonState _buttonState;

  @override
  void initState() {
    _buttonState = ButtonState.Reset;
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return SubmitButton(
        width: widget.width,
        text: widget.type,
        buttonState: _buttonState,
        onPressed: widget.valid
            ? () {
                setState(() {
                  _buttonState = ButtonState.Loading;
                });
                widget.submit();
              }
            : null);
  }
}
