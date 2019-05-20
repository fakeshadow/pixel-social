import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/TopicInputBlocs.dart';

import 'package:pixel_flutter_web/style/colors.dart';
import 'package:pixel_flutter_web/style/text.dart';

class InputPage extends StatefulWidget with env {
  final Function onWillPop;
  final Function onCancelButtonPressed;

  InputPage({@required this.onWillPop, @required this.onCancelButtonPressed});

  @override
  _InputPageState createState() => _InputPageState();
}

class _InputPageState extends State<InputPage> {
  final titleController = TextEditingController();
  final bodyController = TextEditingController();

  TopicInputBloc _topicInputBloc;

  @override
  void initState() {
    _topicInputBloc = TopicInputBloc();
    titleController.addListener(_onTitleChange);
    bodyController.addListener(_onBodyChange);
    super.initState();
  }

  @override
  void dispose() {
    _topicInputBloc.dispose();
    titleController.dispose();
    bodyController.dispose();
    super.dispose();
  }

  void _onTitleChange() {
    _topicInputBloc.dispatch(TitleChanged(title: titleController.text));
  }

  void _onBodyChange() {
    _topicInputBloc.dispatch(BodyChanged(body: bodyController.text));
  }

  void _submit() {
    print(titleController.text);
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _topicInputBloc,
        builder: (context, TopicInputState state) {
          return WillPopScope(
              onWillPop: () {
                if (state.title.isEmpty && state.body.isEmpty) {
                  Navigator.pop(context);
                } else {
                  return widget.onWillPop();
                }
              },
              child: AlertDialog(
                title: Text('Start a new topic'),
                contentPadding: EdgeInsets.all(16),
                content: Container(
                  width: MediaQuery.of(context).size.width <
                          widget.BREAK_POINT_WIDTH_SM
                      ? MediaQuery.of(context).size.width
                      : widget.BREAK_POINT_WIDTH_SM,
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[titleInput(state), bodyInput(state)],
                  ),
                ),
                actions: <Widget>[
                  FlatButton(
                    onPressed: widget.onCancelButtonPressed,
                    child: Text(
                      'Cancel',
                      style: recoverButtonStyle,
                    ),
                  ),
                  RaisedButton(
                    color: primaryColor,
                    onPressed: () => _submit(),
                    child: Text(
                      'Submit',
                      style: submitButtonStyle.copyWith(fontSize: 16),
                    ),
                  )
                ],
              ));
        });
  }

  Widget titleInput(TopicInputState state) {
    return TextFormField(
      controller: titleController,
      autofocus: true,
      maxLength: widget.MAX_TITLE_LENGTH,
      keyboardType: TextInputType.multiline,
      maxLines: null,
      decoration: InputDecoration(
          border: OutlineInputBorder(),
          labelText: 'Title',
          hintText: 'please input your topic title'),
      autovalidate: true,
      validator: (_) {
        return state.isTitleValid || state.title.isEmpty
            ? null
            : 'Title have to be at least 8 characters';
      },
    );
  }

  Widget bodyInput(TopicInputState state) {
    return TextFormField(
      controller: bodyController,
      autofocus: false,
      maxLength: widget.MAX_TEXT_LENGTH,
      keyboardType: TextInputType.multiline,
      maxLines: null,
      decoration: InputDecoration(
          border: OutlineInputBorder(),
          labelText: 'Body',
          hintText: 'please input your topic body'),
      autovalidate: true,
      validator: (_) {
        return state.isBodyValid || state.body.isEmpty
            ? null
            : 'body have to be at least 8 characters';
      },
    );
  }
}
