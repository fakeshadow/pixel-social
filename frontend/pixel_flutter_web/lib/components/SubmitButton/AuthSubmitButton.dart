import 'package:flutter_web/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/UserBlocs.dart';

import 'package:pixel_flutter_web/components/SubmitButton/SubmitButton.dart';

class AuthSubmitButton extends StatefulWidget {
  final String type;
  final double width;
  final Function submit;
  final bool valid;

  AuthSubmitButton(
      {@required this.valid,
      @required this.type,
      @required this.width,
      @required this.submit});

  @override
  _AuthSubmitButtonState createState() => _AuthSubmitButtonState();
}

class _AuthSubmitButtonState extends State<AuthSubmitButton> {
  ButtonState _buttonState;

  @override
  void initState() {
    _buttonState = ButtonState.Reset;
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocListener(
        bloc: BlocProvider.of<UserBloc>(context),
        listener: (BuildContext context, UserState state) {
          if (state is Failure) {
            setState(() => _buttonState = ButtonState.Reset);
          }
          if (state is UserLoaded) {
            setState(() => _buttonState = ButtonState.Success);
          }
        },
        child: SubmitButton(
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
                : null));
  }
}
