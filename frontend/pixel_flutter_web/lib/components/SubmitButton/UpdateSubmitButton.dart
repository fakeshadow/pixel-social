import 'package:flutter_web/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/UpdateBlocs.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';

import 'package:pixel_flutter_web/components/SubmitButton/SubmitButton.dart';

class UpdateSubmitButton extends StatefulWidget {
  final UpdateBloc updateBloc;
  final String type;
  final double width;
  final Function submit;
  final bool valid;

  UpdateSubmitButton(
      {@required this.valid,
      @required this.type,
      @required this.width,
      @required this.submit,
      @required this.updateBloc});

  @override
  _UpdateSubmitButtonState createState() => _UpdateSubmitButtonState();
}

class _UpdateSubmitButtonState extends State<UpdateSubmitButton> {
  ButtonState _buttonState;

  @override
  void initState() {
    _buttonState = ButtonState.Reset;
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocListener(
        bloc: widget.updateBloc,
        listener: (BuildContext context, UpdateState state) {
          if (state is GotError) {
            BlocProvider.of<ErrorBloc>(context)
                .dispatch(GetError(error: state.error));
            setState(() => _buttonState = ButtonState.Reset);
          }
          if (state is GotTopic) {
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
