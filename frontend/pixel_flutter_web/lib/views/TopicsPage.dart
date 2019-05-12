import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/TopicBlocs.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';

import 'package:pixel_flutter_web/components/FloatingAppBar.dart';
import 'package:pixel_flutter_web/components/GeneralBackground.dart';
import 'package:pixel_flutter_web/components/UserDrawer.dart';
import 'package:pixel_flutter_web/models/Category.dart';

class TopicsPage extends StatefulWidget {
  final Category category;

  TopicsPage({@required this.category});

  @override
  _TopicsPageState createState() => _TopicsPageState();
}

class _TopicsPageState extends State<TopicsPage> {
  int _categoryId;
  String _categoryName;
  String _categoryThumbnail;
  TopicBloc _topicBloc;
  ErrorBloc _errorBloc;
  final _scrollController = ScrollController();
  final _scrollThreshold = 200.0;

  @override
  void initState() {
    _categoryId = widget.category.id != null ? widget.category.id : 1;
    _categoryName =
        widget.category.name != null ? widget.category.name : 'PixelShare';
    _categoryThumbnail = widget.category.thumbnail;
    _topicBloc = TopicBloc();
    _errorBloc = BlocProvider.of<ErrorBloc>(context);
//    _errorBloc.dispatch(HideSnack());
    _topicBloc.dispatch(GetTopics(categoryId: _categoryId));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _topicBloc,
        builder: (BuildContext context, TopicState state) {
          return WillPopScope(
            onWillPop: _hideSnack,
            child: Hero(
              tag: _categoryName,
              child: Scaffold(
                endDrawer: UserDrawer(),
                body: BlocListener(
                  bloc: _errorBloc,
                  listener: (BuildContext context, ErrorState state) async {
                    snackbarController(context, state);
                  },
                  child: Stack(
                    children: <Widget>[
                      GeneralBackground(),
                      CustomScrollView(
                          controller: _scrollController,
                          slivers: <Widget>[
                            FloatingAppBar(title: _categoryName),
                            TopicList(state)
                          ])
                    ],
                  ),
                ),
              ),
            ),
          );
        });
  }

  snackbarController(BuildContext context, ErrorState state) async {
    if (state is NoSnack) {
      Scaffold.of(context).hideCurrentSnackBar();
    } else if (state is ShowSuccess) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.green,
        content: Text(
          state.success,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    } else if (state is ShowError) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.deepOrangeAccent,
        content: Text(
          state.error,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    }
  }

  @override
  void dispose() {
    _errorBloc.dispatch(HideSnack());
    _topicBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  Future<bool> _hideSnack() async {
    _errorBloc.dispatch(HideSnack());
    return true;
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicBloc.dispatch(GetTopics(categoryId: _categoryId));
    }
  }
}

class TopicList extends StatelessWidget {
  final state;

  TopicList(this.state);

  @override
  Widget build(BuildContext context) {
    final _errorBloc = BlocProvider.of<ErrorBloc>(context);
    if (state is TopicError) {
      _errorBloc.dispatch(GetError(error: 'No topics found'));
      return CenterLoader();
    }
    if (state is TopicLoaded) {
      if (state.topics.isEmpty) {
        _errorBloc.dispatch(GetSuccess(success: 'No more topics'));
        return CenterLoader();
      }
      return SliverList(
        delegate: SliverChildBuilderDelegate((context, index) {
          return index >= state.topics.length
              ? BottomLoader()
              : TopicView(state.topics[index]);
        },
            childCount: state.hasReachedMax
                ? state.topics.length
                : state.topics.length + 1),
      );
    } else {
      return CenterLoader();
    }
  }
}
