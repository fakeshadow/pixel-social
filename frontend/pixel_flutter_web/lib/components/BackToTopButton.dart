import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/FloatingButtonBlocs.dart';

class BackToTopButton extends StatelessWidget with env {
  final Function onPressed;

  BackToTopButton({this.onPressed});

  @override
  Widget build(BuildContext context) {
    return Padding(
        padding: MediaQuery.of(context).size.width < BREAK_POINT_WIDTH_SM
            ? EdgeInsets.all(0)
            : EdgeInsets.only(
                right: MediaQuery.of(context).size.width * 0.4 - 50),
        child: BlocBuilder(
          bloc: BlocProvider.of<FloatingButtonBloc>(context),
          builder: (context, state) {
            if (state is IsVisible) {
              return AnimatedContainer(
                  duration: Duration(milliseconds: 400),
                  width: state.isVisible ? 40 : 0,
                  child: state.isVisible
                      ? FloatingActionButton(
                          child: Icon(Icons.arrow_upward),
                          mini: true,
                          tooltip: 'Go back to top',
                          onPressed: onPressed,
                        )
                      : Container());
            }
          },
        ));
  }
}
