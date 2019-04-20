import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/RegisterBlocs.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';

import 'package:pixel_flutter/style/colors.dart';
import 'package:pixel_flutter/style/text.dart';

class SubmitAnimatedButton extends StatefulWidget {
  final RegisterState state;
  final String type;
  final Function submit;

  SubmitAnimatedButton(
      {@required this.state, @required this.type, @required this.submit});

  @override
  _SubmitAnimatedButtonState createState() => _SubmitAnimatedButtonState();
}

class _SubmitAnimatedButtonState extends State<SubmitAnimatedButton> {
  ButtonState _buttonState;

  @override
  void initState() {
    _buttonState = ButtonState.Reset;
    super.initState();
  }

  void submitted() {
    setState(() => _buttonState = ButtonState.Loading);
    widget.submit();
  }

  @override
  Widget build(BuildContext context) {
    // ToDo: use error bloc to handle this reset state;
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
      child: AnimatedContainer(
          duration: Duration(milliseconds: 350),
          width: _buttonState == ButtonState.Reset ? 275 : 30,
          height: 30,
          child: RaisedButton(
              clipBehavior: Clip.antiAlias,
              shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(25.0)),
              disabledColor: Colors.black12,
              color: primaryColor,
              onPressed:
                  widget.state.isRegisterValid && widget.type == 'Register'
                      ? submitted
                      : widget.state.isLoginValid && widget.type == 'Login'
                          ? submitted
                          : null,
              child: _buttonState == ButtonState.Reset
                  ? Text(widget.type, style: submitButtonStyle)
                  : _buttonState == ButtonState.Loading
                      ? SizedBox(
                          height: 20,
                          width: 20,
                          child: CircularProgressIndicator(
                            valueColor: AlwaysStoppedAnimation<Color>(Colors.white),
                            strokeWidth: 2,
                          ))
                      : SizedBox(
                          height: 20,
                          width: 20,
                          child: Icon(Icons.check,color: Colors.white,),
                        ))),
    );
  }
}

enum ButtonState {
  Loading,
  Success,
  Reset,
}
